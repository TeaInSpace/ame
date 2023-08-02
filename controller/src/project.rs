use ame::{
    ctrl::AmeResource,
    custom_resources::{
        new_task::{Task, TaskBuilder},
        project::{generate_task_name, get_latest_model_version, Project},
    },
};

use ame::{error::AmeError, Result};

use ame::grpc::{task_status::Phase, ModelDeploymentCfg, ModelTrainingCfg, TaskStatus, TriggerCfg};

use chrono::Utc;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use k8s_openapi::api::{apps::v1::Deployment, core::v1::Service, networking::v1::Ingress};
use kube::{
    api::{ListParams, Patch, PatchParams},
    runtime::{controller::Action, finalizer, Controller},
    Api, Client, ResourceExt,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

static PROJECT_CONTROLLER: &str = "projects.ame.teainspace.com";

#[derive(Clone)]
struct Context {
    cfg: ProjectControllerCfg,
    pub client: Client,
}
impl Context {
    fn new(client: Client, cfg: ProjectControllerCfg) -> Self {
        Self { client, cfg }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct ProjectControllerCfg {
    pub namespace: Option<String>,
    pub deployment_image: Option<String>,
    pub mlflow_url: Option<String>,
    model_deployment_ingress: Option<Ingress>,
    model_ingress_annotations: Option<BTreeMap<String, String>>,
    model_ingress_host: Option<String>,
}

impl ProjectControllerCfg {
    pub fn from_env() -> Result<Self> {
        let prefix = "AME";
        Ok(ProjectControllerCfg {
            namespace: std::env::var(format!("{prefix}_NAMESPACE")).ok(),
            deployment_image: std::env::var("EXECUTOR_IMAGE").ok(),
            model_deployment_ingress: serde_yaml::from_str(
                &std::env::var(format!("{prefix}_MODEL_DEPLOYMENT_INGRESS"))
                    .unwrap_or("".to_string()),
            )
            .ok(),
            model_ingress_annotations: Some(BTreeMap::new()),
            model_ingress_host: std::env::var(format!("{prefix}_MODEL_INGRESS_HOST")).ok(),
            mlflow_url: std::env::var(format!("{prefix}_MLFLOW_URL")).ok(),
        })
    }
}

impl ProjectControllerCfg {
    pub fn new(namespace: Option<String>) -> Self {
        Self {
            namespace,
            deployment_image: None,
            mlflow_url: None,
            model_deployment_ingress: None,
            model_ingress_annotations: None,
            model_ingress_host: None,
        }
    }
}

async fn reconcile(project: Arc<Project>, ctx: Arc<Context>) -> Result<Action> {
    let projects = if let Some(ref namespace) = ctx.cfg.namespace {
        Api::<Project>::namespaced(ctx.client.clone(), namespace)
    } else {
        Api::<Project>::namespaced(ctx.client.clone(), "ame-system")
    };

    info!("reconciling project {}", project.name_any());

    Ok(
        finalizer(&projects, PROJECT_CONTROLLER, project, |event| async {
            match event {
                finalizer::Event::Apply(project) => {
                    apply(&project, &projects, ctx.client.clone(), ctx).await
                }
                finalizer::Event::Cleanup(project) => cleanup(&project, &projects).await,
            }
        })
        .await?,
    )
}

async fn apply(
    project: &Project,
    _projects: &Api<Project>,
    client: Client,
    ctx: Arc<Context>,
) -> Result<Action> {
    let project_status = project.status.clone().unwrap_or_default();
    let tasks = Api::<Task>::namespaced(client.clone(), &project.namespace().unwrap());
    let deployments = Api::<Deployment>::namespaced(client.clone(), &project.namespace().unwrap());
    let services = Api::<Service>::namespaced(client.clone(), &project.namespace().unwrap());
    let ingresses = Api::<Ingress>::namespaced(client, &project.namespace().unwrap());

    let Some(project_oref) = project.gen_owner_ref() else {
        return Err(AmeError::FailedToCreateOref(project.name_any()));
    };

    if project.spec.enable_triggers.unwrap_or(false) {
        info!("checking for triggered tasks");
        let instant = Utc::now();

        for task in project.spec.cfg.tasks.iter() {
            if let Some(TriggerCfg {
                schedule: Some(ref schedule),
            }) = task.triggers
            {
                // TODO this parser is brittle and panics if len < 3;
                let schedule = match cron_parser::parse(schedule, &instant) {
                    Ok(schedule) => schedule,
                    e @ Err(_) => {
                        error!(
                            "failed to pass cron schedule for task {}: {e:?}",
                            task.name.as_ref().unwrap_or(&"".to_string())
                        );
                        continue;
                    }
                };

                if schedule.signed_duration_since(instant).num_seconds() < 120 {
                    let mut task_builder = TaskBuilder::from_cfg(task.clone());
                    task_builder.add_owner_reference(project_oref.clone());

                    task_builder.set_name(generate_task_name(
                        project.name_any(),
                        task.name.clone().unwrap_or("".to_string()),
                    ));
                    task_builder.set_project(project.spec.cfg.name.clone());

                    let task = task_builder.build();

                    let Some(ref task_name) = task.metadata.name else {
                        error!("task is missing a name in project {}", project.name_any());
                        continue;
                    };

                    let res = tasks
                        .patch(
                            task_name,
                            &PatchParams::apply(PROJECT_CONTROLLER),
                            &Patch::Apply(task.clone()),
                        )
                        .await;

                    if let e @ Err(_) = res {
                        error!(
                            "failed to patch task {} in project {}: {:?}",
                            task.name_any(),
                            project.name_any(),
                            e
                        );
                        continue;
                    };
                }
            }
        }
    } else {
        debug!("triggers are disabled");
    }

    // Note task controller is not updating task status to succeeded.
    // Why is controller not reacting owned task status changing?
    for model in project.spec.cfg.models.iter() {
        info!(
            "reconciling model {} in project {}",
            model.name, project.spec.cfg.name
        );
        if let Some(ModelTrainingCfg {
            task: Some(ref _task),
            ..
        }) = model.training
        {
            let Ok(training_task) = project.generate_model_training_task(&model.name) else {
                continue;
            };

            debug!("Patching training task: {:?}", training_task);

            tasks
                .patch(
                    &training_task.name_any(),
                    &PatchParams::apply(PROJECT_CONTROLLER),
                    &Patch::Apply(training_task),
                )
                .await?;
        }

        if let Some(ModelDeploymentCfg { ref image, .. }) = model.deployment {
            let deployment_image = if let Some(image) = image {
                Some(image.clone())
            } else {
                ctx.cfg.deployment_image.clone()
            };

            let Some(deployment_image) = deployment_image else {
                error!(
                    "missing deployment image for model {} in project {}",
                    model.name,
                    project.name_any()
                );
                continue;
            };

            let Some(mlflow_url) = ctx.cfg.mlflow_url.clone() else {
                error!("missing MLflow URL, skipping deployment");
                continue;
            };

            let model_source = match get_latest_model_version(model, mlflow_url).await {
                Ok(ms) => ms,
                Err(e) => {
                    error!("failed to get latest model version, skipping deployment error: {e}");
                    continue;
                }
            };

            let mut model_status = project_status
                .models
                .get(&model.name)
                .cloned()
                .unwrap_or_default();

            if model_status
                .latest_validated_model_version
                .as_ref()
                .map(|s| s != &model_source.source)
                .unwrap_or(true)
                && model.validation_task.is_some()
            {
                debug!(
                    "Generating validation task for model {} in project {}",
                    model.name, project.spec.cfg.name
                );

                let val_task = match project
                    .generate_validation_task(model, model_source.version.clone())
                {
                    Ok(t) => Some(t),
                    Err(e) => {
                        error!("failed to generate validaion task for model {} in project {}, aborting model validation and deployment: {}", model.name, project.spec.cfg.name, e);
                        None
                    }
                };

                let Some(ref val_task) = val_task else {
                    continue;
                };

                let Some(ref task_name) = val_task.metadata.name else {
                    error!("validation task for model {} in project {} is missing a name, aborting model validation and deployment", model.name, project.spec.cfg.name);
                    continue;
                };

                info!(
                    "patching validation task for model {} in project {}",
                    model.name,
                    project.name_any()
                );

                // TODO; we need to look at the name generation for validated model tasks.
                tasks
                    .patch(
                        task_name,
                        &PatchParams::apply(PROJECT_CONTROLLER),
                        &Patch::Apply(val_task),
                    )
                    .await?;

                // TODO: what about when validation has failed?
                if let Ok(Task {
                    status:
                        Some(TaskStatus {
                            phase: Some(Phase::Succeeded(_)),
                        }),
                    ..
                }) = tasks.get_status(task_name).await
                {
                    info!(
                        "model {} in project {} is validated for version {}",
                        model.name,
                        project.name_any(),
                        model_source.version
                    );
                    model_status.latest_validated_model_version = Some(model_source.source.clone());
                } else {
                    info!("model {} in project {} is not validated for version {}, skipping deployment", model.name, project.name_any(), model_source.version);
                    continue;
                }
            }

            let deployment = model
                .generate_model_deployment(deployment_image.clone(), model_source.source)
                .await;
            match deployment {
                Ok(deployment) => {
                    let Some(ref name) = deployment.metadata.name else {
                        error!("Deployment object for model {} in project {} is missing a name, skipping deployment", model.name, project.name_any());
                        continue;
                    };

                    info!("Patching model deployment");

                    // TODO: adjust error handling to avoid breaking out of the reconciliation for errors on a single object.
                    deployments
                        .patch(
                            name,
                            &PatchParams::apply(PROJECT_CONTROLLER),
                            &Patch::Apply(deployment.clone()),
                        )
                        .await?;
                }
                Err(e) => error!(
                    "Failed to generated model deployment for {} in project {} error: {}",
                    model.name,
                    project.name_any(),
                    e
                ),
            };

            let service = model.generate_model_service();

            match service {
                Ok(service) => {
                    let Some(ref name) = service.metadata.name else {
                        error!("Service object for model {} in project {} is missing a name, skipping service", model.name, project.name_any());
                        continue;
                    };

                    info!("Patching model service");
                    services
                        .patch(
                            name,
                            &PatchParams::apply(PROJECT_CONTROLLER),
                            &Patch::Apply(service.clone()),
                        )
                        .await?;
                }
                Err(e) => error!(
                    "Failed to generated service for model {} in project {}: {}",
                    model.name,
                    project.name_any(),
                    e
                ),
            };

            // TODO: put some thought into how the project name is used and the path for a model created.
            let ingress = model.generate_model_ingress(
                ctx.cfg.model_ingress_host.clone().unwrap_or("".to_string()),
                None,
                project.name_any(),
            );

            match ingress {
                Ok(ingress) => {
                    let Some(ref name) = ingress.metadata.name else {
                        error!("Ingress object for model {} in project {} is missing a name, skipping ingress", model.name, project.name_any());
                        continue;
                    };

                    info!("Patching model ingress");
                    ingresses
                        .patch(
                            name,
                            &PatchParams::apply(PROJECT_CONTROLLER),
                            &Patch::Apply(ingress.clone()),
                        )
                        .await?;
                }
                Err(e) => error!(
                    "Failed to generated ingress for model {} in project {}: {}",
                    model.name,
                    project.name_any(),
                    e
                ),
            }
        }
    }

    Ok(Action::requeue(Duration::from_secs(60)))
}

pub async fn cleanup(project: &Project, _projects: &Api<Project>) -> Result<Action> {
    if !project.deletion_approved() {
        info!(
            "Blocking deletion as it was not approved for {}",
            project.name_any()
        );
        return Err(AmeError::DeletionNotApproved(project.name_any()));
    }

    Ok(Action::await_change())
}

fn error_policy(_project: Arc<Project>, error: &AmeError, _ctx: Arc<Context>) -> Action {
    error!("failed to reconcile: {:?}", error);
    Action::requeue(Duration::from_secs(5 * 60))
}

pub async fn start_project_controller(
    client: Client,
    config: ProjectControllerCfg,
) -> Result<BoxFuture<'static, ()>> {
    let context = Arc::new(Context::new(client.clone(), config.clone()));

    let projects = if let Some(ref namespace) = config.namespace {
        Api::<Project>::namespaced(client.clone(), namespace)
    } else {
        Api::<Project>::all(client.clone())
    };

    let tasks = if let Some(ref namespace) = config.namespace {
        Api::<Task>::namespaced(client.clone(), namespace)
    } else {
        Api::<Task>::all(client.clone())
    };

    info!("Starting Project controller");
    debug!("Project controller cfg: {:?}", config);

    Ok(Controller::new(projects, ListParams::default())
        .owns(tasks, ListParams::default())
        .run(reconcile, error_policy, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .boxed())
}

#[cfg(test)]
mod test {
    use ame::{
        custom_resources::project::{Project, ProjectSpec},
        grpc::ProjectCfg,
        Result,
    };
    use kube::{
        api::{DeleteParams, Patch, PatchParams, PostParams},
        core::ObjectMeta,
        Api, Client,
    };

    use super::*;

    #[tokio::test]
    #[ignore = "requires a k8s cluster"]
    async fn can_block_deletion() -> Result<()> {
        let client = Client::try_default().await?;
        let namespace = "default".to_string();
        let projects = Api::<Project>::namespaced(client.clone(), &namespace);

        let ctx = Context {
            client,
            cfg: ProjectControllerCfg::new(Some(namespace)),
        };

        let project = Project {
            metadata: ObjectMeta {
                generate_name: Some("myproject".to_string()),
                finalizers: Some(vec![super::PROJECT_CONTROLLER.to_string()]),
                ..ObjectMeta::default()
            },
            spec: ProjectSpec {
                cfg: ProjectCfg {
                    name: "myproject".to_string(),
                    ..ProjectCfg::default()
                },
                deletion_approved: false,
                enable_triggers: Some(false),
            },
            status: None,
        };

        let project = projects.create(&PostParams::default(), &project).await?;

        reconcile(Arc::new(project.clone()), Arc::new(ctx.clone())).await?;

        let project = projects.get(&project.name_any()).await?;

        reconcile(Arc::new(project.clone()), Arc::new(ctx.clone())).await?;

        projects
            .delete(&project.name_any(), &DeleteParams::default())
            .await?;

        let mut project = projects.get(&project.name_any()).await?;
        reconcile(Arc::new(project.clone()), Arc::new(ctx.clone()))
            .await
            .unwrap_err();

        project.approve_deletion();

        let project = projects
            .patch(
                &project.name_any(),
                &PatchParams::default(),
                &Patch::Merge(project),
            )
            .await?;

        projects
            .delete(&project.name_any(), &DeleteParams::default())
            .await?;

        Ok(())
    }
}
