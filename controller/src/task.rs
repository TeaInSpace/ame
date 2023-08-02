use std::{
    sync::Arc,
    time::{self, Duration},
};

use ame::{
    ctrl::AmeResource,
    custom_resources::{
        argo::{Workflow, WorkflowPhase},
        data_set::{DataSet, DataSetPhase, DataSetStatus},
        find_project,
        new_task::{build_workflow, resolve_task_templates, Task},
        project::{local_name, project_name, Project},
        task_ctrl::TaskCtrl,
    },
    error::AmeError,
    grpc::{task_status::Phase, TaskPhaseFailed, TaskPhaseRunning, TaskPhaseSucceeded, TaskStatus},
    Result,
};
use envconfig::Envconfig;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use kube::{
    api::{ListParams, Patch, PatchParams},
    runtime::{controller::Action, finalizer, Controller},
    Api, Client, ResourceExt,
};
use tracing::{debug, error, info};

static TASK_CONTROLLER: &str = "tasks.ame.teainspace.com";

#[derive(Clone)]
struct Context {
    cfg: TaskControllerCfg,
    client: Client,
}

impl Context {
    fn new(client: Client, cfg: TaskControllerCfg) -> Self {
        Self { client, cfg }
    }
}

#[derive(Clone, Envconfig, Debug)]
pub struct TaskControllerCfg {
    #[envconfig(
        from = "AME_EXECUTOR_IMAGE",
        default = "main.localhost:45373/ame-executor:latest"
    )]
    pub executor_image: String,
    #[envconfig(from = "NAMESPACE")]
    pub namespace: Option<String>,
    #[envconfig(from = "AME_SERVICE_ACCOUNT", default = "ame-task")]
    pub service_account: String,
}

async fn reconcile(task: Arc<Task>, ctx: Arc<Context>) -> Result<Action> {
    info!("reconciling task {}", task.name_any());

    let tasks = if let Some(ref namespace) = ctx.cfg.namespace {
        Api::<Task>::namespaced(ctx.client.clone(), namespace)
    } else {
        todo!("we need to handle this case better??");
    };

    Ok(finalizer(&tasks, TASK_CONTROLLER, task, |event| async {
        match event {
            finalizer::Event::Apply(task) => apply(&task, &ctx).await,
            finalizer::Event::Cleanup(task) => cleanup(&task, &tasks).await,
        }
    })
    .await?)
}

// TODO: do not allow nonexistent fields in project.yaml.

async fn apply(task: &Task, ctx: &Context) -> Result<Action> {
    let (tasks, workflows, projects, data_sets) = if let Some(ref namespace) = ctx.cfg.namespace {
        (
            Api::<Task>::namespaced(ctx.client.clone(), namespace),
            Api::<Workflow>::namespaced(ctx.client.clone(), namespace),
            Api::<Project>::namespaced(ctx.client.clone(), namespace),
            Api::<DataSet>::namespaced(ctx.client.clone(), namespace),
        )
    } else {
        todo!("we need to handle this case better??");
    };

    let task_ctrl = TaskCtrl::new(data_sets.clone(), projects.clone());

    debug!("checking datasets for task {:?}", task.name_any());

    let project = projects.get(&task.parent_project_name()?).await?;

    if !task.spec.cfg.data_sets.is_empty() {
        info!(
            "reconciling data sets for task {}",
            task.spec.cfg.name.as_ref().unwrap()
        );

        let mut ds_statuses: Vec<Option<DataSetStatus>> = vec![];

        debug!("reconciling datasets {:?}", task.spec.cfg.data_sets);

        for ds in task.spec.cfg.data_sets.iter() {
            let ds_name = local_name(ds.clone());

            let ds_project = if let Some(project_name) = project_name(ds.clone()) {
                find_project(projects.clone(), project_name, "".to_string())
                    .await
                    .or(Err(AmeError::MissingProject(0)))?
            } else {
                project.clone()
            };

            let mut data_set = match ds_project.generate_data_set(ds_name.clone()) {
                Ok(ds) => ds,
                Err(e) => {
                    error!(
                        "failed to reconcile data set {} for project {}: {}",
                        ds_name,
                        ds_project.name_any(),
                        e
                    );
                    continue;
                }
            };
            let Some(task_oref) = task.gen_owner_ref() else {
                error!(
                    "failed to generate owner reference for task {} stopping data set creation",
                    task.name_any()
                );
                continue;
            };

            data_set.owner_references_mut().push(task_oref);

            debug!("patching dataset: {}", ds);

            let ds_obj_name =
                data_set
                    .metadata
                    .name
                    .clone()
                    .ok_or(AmeError::ReconcilitationFailure(
                        "Task".to_string(),
                        task.name_any(),
                        format!(
                            "data set  {} in project {} is missing a name",
                            ds, project.spec.cfg.name
                        ),
                    ))?;

            let data_set = data_sets
                .patch(
                    &ds_obj_name,
                    &PatchParams::apply(TASK_CONTROLLER).force(),
                    &Patch::Apply(data_set),
                )
                .await?;

            ds_statuses.push(data_set.status);
        }

        for stat in ds_statuses {
            let Some(stat) = stat else {
                info!("waiting for datasets to complete");
                return Ok(Action::requeue(Duration::from_secs(10)));
            };

            match stat.phase {
                Some(DataSetPhase::Ready { .. }) => continue,
                Some(DataSetPhase::Failed { .. }) => {
                    error!(
                        "Data set has failed, can not schedule Task {}",
                        task.spec
                            .cfg
                            .name
                            .as_ref()
                            .unwrap_or(&"unknown name".to_string())
                    );
                }
                _ => {
                    info!("waiting for datasets to complete");
                    return Ok(Action::requeue(Duration::from_secs(10)));
                }
            }
        }
    }

    let task_ctx = task_ctrl
        .gather_task_ctx(
            task,
            ctx.cfg.executor_image.to_string(),
            ctx.cfg.service_account.clone(),
        )
        .await?;

    debug!("resolving task {}", task.name_any());

    let resolved_task = resolve_task_templates(task.clone(), project, projects).await?;

    debug!("resolved task {:?}", task.spec.cfg);

    let workflow = build_workflow(resolved_task, task_ctx)?;

    debug!("patching workflow for task {:?} ", task.name_any(),);

    let workflow = workflows
        .patch(
            &workflow.name_any(),
            &PatchParams::apply(TASK_CONTROLLER).force(),
            &Patch::Apply(&workflow),
        )
        .await?;

    debug!("workflow phase: {:?}", workflow.status);

    let phase = workflow
        .status
        .as_ref()
        .map(|s| match s.phase {
            WorkflowPhase::Pending | WorkflowPhase::Running | WorkflowPhase::Error => {
                Phase::Running(TaskPhaseRunning {
                    workflow_name: workflow.name_any(),
                })
            }
            WorkflowPhase::Failed => Phase::Failed(TaskPhaseFailed {
                workflow_name: workflow.name_any(),
            }),
            WorkflowPhase::Succeeded => Phase::Succeeded(TaskPhaseSucceeded {
                workflow_name: workflow.name_any(),
            }),
        })
        .unwrap_or(Phase::Running(TaskPhaseRunning {
            workflow_name: workflow.name_any(),
        }));

    let mut task = task.clone();

    task.status = Some(TaskStatus { phase: Some(phase) });
    task.metadata.managed_fields = None;

    debug!("patching status for task {}", task.name_any());
    tasks
        .patch_status(
            &task.name_any(),
            &PatchParams::apply(TASK_CONTROLLER).force(),
            &Patch::Apply(task),
        )
        .await?;

    Ok(Action::requeue(std::time::Duration::from_secs(60)))
}

pub async fn cleanup(task: &Task, _tasks: &Api<Task>) -> Result<Action> {
    info!("cleanup dataset: {}", task.name_any());

    if task.spec.deletion_approved {
        Ok(Action::requeue(time::Duration::from_secs(300)))
    } else {
        Err(AmeError::ApiError("deletion not approved".to_string()))
    }
}

pub async fn start_task_controller(
    client: Client,
    config: TaskControllerCfg,
) -> Result<BoxFuture<'static, ()>> {
    info!("Start Task controller");
    let context = Arc::new(Context::new(client.clone(), config.clone()));

    let tasks = if let Some(ref namespace) = config.namespace {
        Api::<Task>::namespaced(client.clone(), namespace)
    } else {
        Api::<Task>::all(client.clone())
    };

    let workflow = if let Some(ref namespace) = config.namespace {
        Api::<Workflow>::namespaced(client, namespace)
    } else {
        Api::<Workflow>::all(client)
    };

    Ok(Controller::new(tasks, ListParams::default())
        .owns(workflow, ListParams::default())
        .run(reconcile, error_policy, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .boxed())
}

fn error_policy(_src: Arc<Task>, error: &AmeError, _ctx: Arc<Context>) -> Action {
    error!("failed to reconcile: {:?}", error);
    Action::requeue(Duration::from_secs(5 * 60))
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;

    use ame::{
        custom_resources::{
            argo::Workflow,
            new_task::{ProjectSource, TaskSpec},
        },
        grpc::{
            task_cfg::Executor, task_status::Phase, PoetryExecutor, ProjectCfg, TaskCfg,
            TaskPhaseRunning,
        },
        Result,
    };
    use envconfig::Envconfig;

    use kube::{
        api::{DeleteParams, PostParams},
        core::ObjectMeta,
        Api, Client, Resource,
    };

    use super::*;

    #[tokio::test]
    #[ignore = "requires a k8s cluster"]
    async fn can_create_workflow_and_finalize_task() -> Result<()> {
        let client = Client::try_default().await?;
        let namespace = "default".to_string();
        let tasks = Api::<Task>::namespaced(client.clone(), &namespace);
        let workflows = Api::<Workflow>::namespaced(client.clone(), &namespace);
        let projects = Api::<Project>::namespaced(client.clone(), &namespace);

        let mut controller_cfg = TaskControllerCfg::init_from_env().unwrap();
        controller_cfg.namespace = Some(namespace);

        let ctx = Context::new(client, controller_cfg);

        let project = ProjectCfg {
            name: "parentproject".to_string(),
            models: vec![],
            data_sets: vec![],
            tasks: vec![],
            templates: vec![],
            enable_triggers: None,
        };

        let project = Project::from_cfg(project);
        let project = projects.create(&PostParams::default(), &project).await?;

        let task = Task {
            metadata: ObjectMeta {
                generate_name: Some("mytask".to_string()),
                owner_references: Some(vec![project.controller_owner_ref(&()).unwrap()]),
                ..ObjectMeta::default()
            },
            spec: TaskSpec {
                cfg: TaskCfg {
                    name: Some("mytask".to_string()),
                    task_ref: None,
                    executor: Some(Executor::Poetry(PoetryExecutor {
                        python_version: "3.11".to_string(),
                        command: "python train.py".to_string(),
                    })),
                    resources: BTreeMap::new(),
                    data_sets: Vec::new(),
                    from_template: None,
                    artifact_cfg: None,
                    triggers: None,
                    env: vec![],
                    secrets: vec![],
                },
                source: Some(ProjectSource::Ame {
                    path: "somepath".to_string(),
                }),
                deletion_approved: false,
                project: None,
            },
            status: None,
        };

        let task = tasks.create(&PostParams::default(), &task).await?;

        reconcile(Arc::new(task.clone()), Arc::new(ctx.clone())).await?;

        let task = tasks.get(&task.name_any()).await?;

        reconcile(Arc::new(task.clone()), Arc::new(ctx.clone())).await?;

        let task_status = tasks.get_status(&task.name_any()).await?.status.unwrap();

        let Phase::Running(TaskPhaseRunning { workflow_name }) = task_status.phase.unwrap() else {
            panic!("task was not running");
        };

        let workflow = workflows.get(&workflow_name).await?;

        let mut settings = insta::Settings::clone_current();

        settings.add_filter(
            &format!("{}.+", task.clone().metadata.generate_name.unwrap()),
            "redacted",
        );

        let _guard = settings.bind_to_scope();
        insta::assert_yaml_snapshot!(&workflow.spec);

        tasks
            .delete(&task.name_any(), &DeleteParams::default())
            .await?;

        let data_set = tasks.get(&task.name_any()).await?;

        // An error is expected here as the cleanup is rejected.
        reconcile(Arc::new(data_set.clone()), Arc::new(ctx.clone()))
            .await
            .unwrap_err();

        // Deletion should not be possible until deletion has been approved.
        let mut task = tasks.get(&task.name_any()).await?;

        // Approve deletion and verify that the data set is now deleted.
        task.spec.deletion_approved = true;

        tasks
            .patch(
                &task.name_any(),
                &PatchParams::default(),
                &Patch::Merge(task.clone()),
            )
            .await?;

        reconcile(Arc::new(task.clone()), Arc::new(ctx.clone()))
            .await
            .unwrap();

        // The cluster is allowed a minimum of 100ms to delete the data set
        // object after the finalizer has approved deletion during the last
        // reconcile call.
        tokio::time::sleep(Duration::from_millis(100)).await;

        tasks.get(&task.name_any()).await.unwrap_err();

        Ok(())
    }
}
