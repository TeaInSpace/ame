use ame::custom_resources::{common::parent_project, new_task::Task, project::Project};
use std::{sync::Arc, time::Duration};

use ame::{
    custom_resources::data_set::{DataSet, DataSetPhase, DataSetStatus},
    error::AmeError,
};

use ame::Result;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use kube::api::{ListParams, Patch, PatchParams};

use kube::{
    runtime::{controller::Action, finalizer, Controller},
    Api, Client, Resource, ResourceExt,
};
use std::default::Default;
use tracing::{debug, error, info};

static DATA_SET_CONTROLLER: &str = "datasets.ame.teainspace.com";

#[derive(Clone)]
struct Context {
    client: Client,
    namespace: String,
}

impl From<DataSetControllerCfg> for Context {
    fn from(cfg: DataSetControllerCfg) -> Self {
        Context {
            client: cfg.client,
            namespace: cfg.namespace,
        }
    }
}

#[derive(Clone)]
pub struct DataSetControllerCfg {
    pub client: Client,
    pub namespace: String,
}

async fn reconcile(data_set: Arc<DataSet>, ctx: Arc<Context>) -> Result<Action> {
    let data_sets = Api::<DataSet>::namespaced(ctx.client.clone(), &ctx.namespace);
    let tasks = Api::<Task>::namespaced(ctx.client.clone(), &ctx.namespace);
    let projects = Api::<Project>::namespaced(ctx.client.clone(), &ctx.namespace);

    info!("reconciling data set {}", data_set.name_any());

    Ok(
        finalizer(&data_sets, DATA_SET_CONTROLLER, data_set, |event| async {
            match event {
                finalizer::Event::Apply(data_set) => {
                    apply(&data_set, &data_sets, &tasks, &projects).await
                }
                finalizer::Event::Cleanup(data_set) => cleanup(&data_set).await,
            }
        })
        .await?,
    )
}

fn error_policy(_src: Arc<DataSet>, error: &AmeError, _ctx: Arc<Context>) -> Action {
    error!("failed to reconcile: {:?}", error);
    Action::requeue(Duration::from_secs(5 * 60))
}

pub async fn start_data_set_controller(
    config: impl Into<DataSetControllerCfg>,
) -> Result<BoxFuture<'static, ()>> {
    let config: DataSetControllerCfg = config.into();
    let context = Arc::new(Context::from(config.clone()));

    let data_sets = Api::<DataSet>::namespaced(config.client.clone(), &config.namespace);
    data_sets.list(&ListParams::default()).await.unwrap();

    let tasks = Api::<Task>::namespaced(config.client.clone(), &config.namespace);

    Ok(Controller::new(data_sets, ListParams::default())
        .owns(tasks, ListParams::default())
        .run(reconcile, error_policy, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .boxed())
}

async fn apply(
    data_set: &DataSet,
    data_sets: &Api<DataSet>,
    tasks: &Api<Task>,
    projects: &Api<Project>,
) -> Result<Action> {
    let mut task = data_set.generate_task()?;

    let parent_project = projects
        .get(&parent_project(data_set.owner_references().to_vec())?)
        .await?;

    let mut project_oref = parent_project.controller_owner_ref(&()).unwrap();
    project_oref.controller = Some(false);

    task.owner_references_mut().push(project_oref);

    debug!("patching data set task: {:?}", task);
    let task = tasks
        .patch(
            &task.name_any(),
            &PatchParams::apply(DATA_SET_CONTROLLER),
            &Patch::Apply(&task),
        )
        .await?;

    let status = DataSetStatus {
        phase: Some(DataSetPhase::from_task(task)),
    };

    debug!("patching data set status {:?}  ", status.clone());

    let mut data_set = data_set.clone();

    // let mut data_set = data_set.clone();
    data_set.metadata.managed_fields = None;
    data_set.status = Some(status.clone());

    data_sets
        .patch_status(
            &data_set.name_any(),
            &PatchParams::apply(DATA_SET_CONTROLLER).force(),
            &Patch::Apply(&data_set),
        )
        .await?;

    Ok(Action::requeue(Duration::from_secs(300)))
}

async fn cleanup(data_set: &DataSet) -> Result<Action> {
    info!("cleanup dataset: {}", data_set.name_any());

    if data_set.spec.deletion_approved {
        Ok(Action::requeue(Duration::from_secs(300)))
    } else {
        Err(AmeError::ApiError("deletion not approved".to_string()))
    }
}

#[cfg(test)]
mod test {
    use ame::{
        custom_resources::data_set::DataSetSpec,
        grpc::{DataSetCfg, ProjectCfg, TaskCfg, TaskRef},
    };
    use kube::{
        api::{DeleteParams, PostParams},
        core::ObjectMeta,
        ResourceExt,
    };

    use super::*;
    use std::{collections::BTreeMap, time::Duration};

    #[tokio::test]
    #[ignore = "requires a k8s cluster"]
    async fn can_create_data_set_task_and_finalize_data_set() -> Result<()> {
        let client = Client::try_default().await?;
        let namespace = "default".to_string();
        let data_sets = Api::<DataSet>::namespaced(client.clone(), &namespace);
        let projects = Api::<Project>::namespaced(client.clone(), &namespace);
        let context = super::Context {
            client: client.clone(),
            namespace,
        };
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

        let data_set = DataSet {
            metadata: ObjectMeta {
                generate_name: Some("testdataset2".to_string()),
                finalizers: Some(vec![super::DATA_SET_CONTROLLER.to_string()]),
                owner_references: Some(vec![project.controller_owner_ref(&()).unwrap()]),
                ..ObjectMeta::default()
            },
            spec: DataSetSpec {
                project: None,
                cfg: DataSetCfg {
                    name: "test".to_string(),
                    path: "data".to_string(),
                    task: Some(TaskCfg {
                        name: Some("testname".to_string()),
                        task_ref: Some(TaskRef {
                            name: "test_data_task".to_string(),
                            project: None,
                        }),
                        executor: None,
                        resources: BTreeMap::new(),
                        data_sets: Vec::new(),
                        from_template: None,
                        artifact_cfg: None,
                        triggers: None,
                        env: vec![],
                        secrets: vec![],
                    }),
                    size: None,
                },
                deletion_approved: false,
            },
            status: Some(DataSetStatus {
                phase: Some(DataSetPhase::Pending {}),
            }),
        };

        let data_set = data_sets.create(&PostParams::default(), &data_set).await?;

        reconcile(Arc::new(data_set.clone()), Arc::new(context.clone()))
            .await
            .unwrap();

        // Note that after calling reconcile we have to fetch the object
        // from the cluster, as our local version will not have been updated.
        let data_set = data_sets.get(&data_set.name_any()).await?;

        assert!(data_set.running_task(client.clone()).await.is_some());

        data_sets
            .delete(&data_set.name_any(), &DeleteParams::default())
            .await?;

        let data_set = data_sets.get(&data_set.name_any()).await?;

        // An error is expected here as the cleanup is rejected.
        reconcile(Arc::new(data_set.clone()), Arc::new(context.clone()))
            .await
            .unwrap_err();

        // The cluster is allowed a minimum of 100ms to delete the data set
        // object after the finalizer has approved deletion during the last
        // reconcile call.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Deletion should not be possible until deletion has been approved.
        let mut data_set = data_sets.get(&data_set.name_any()).await.unwrap();

        // Approve deletion and verify that the data set is now deleted.
        data_set.spec.deletion_approved = true;

        data_sets
            .patch(
                &data_set.name_any(),
                &PatchParams::default(),
                &Patch::Merge(data_set.clone()),
            )
            .await?;

        reconcile(Arc::new(data_set.clone()), Arc::new(context.clone()))
            .await
            .unwrap();

        data_sets.get(&data_set.name_any()).await.unwrap_err();

        Ok(())
    }
}
