use ame::custom_resources::task::Task;
use std::sync::Arc;
use std::time::Duration;

use ame::custom_resources::data_set::{DataSet, DataSetPhase, DataSetSpec, DataSetStatus};
use ame::error::AmeError;

use ame::Result;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use kube::api::{ListParams, Patch, PatchParams};
use kube::core::ObjectMeta;
use kube::runtime::controller::Action;
use kube::runtime::{finalizer, Controller};
use kube::{Api, Client, ResourceExt};
use std::default::Default;
use tracing::{error, info};

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

    info!("reconciling data set {}", data_set.name_any());

    Ok(
        finalizer(&data_sets, DATA_SET_CONTROLLER, data_set, |event| async {
            match event {
                finalizer::Event::Apply(data_set) => apply(&data_set, &data_sets, &tasks).await,
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
    data_sets.list(&ListParams::default()).await?;

    let tasks = Api::<Task>::namespaced(config.client.clone(), &config.namespace);

    Ok(Controller::new(data_sets, ListParams::default())
        .owns(tasks, ListParams::default())
        .run(reconcile, error_policy, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .boxed())
}

async fn apply(data_set: &DataSet, data_sets: &Api<DataSet>, tasks: &Api<Task>) -> Result<Action> {
    let task = data_set.generate_task()?;

    let task = tasks
        .patch(
            &task.name_any(),
            &PatchParams::apply(DATA_SET_CONTROLLER),
            &Patch::Apply(&task),
        )
        .await?;

    let status = DataSetStatus {
        phase: DataSetPhase::from_task(task),
    };

    data_sets
        .patch_status(
            &data_set.name_any(),
            &PatchParams::default(),
            &Patch::Merge(&DataSet {
                metadata: ObjectMeta::default(),
                status: Some(status),
                spec: DataSetSpec::default(),
            }),
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
        grpc::{DataSetCfg, TaskCfg, TaskRef},
    };
    use kube::{
        api::{DeleteParams, PostParams},
        core::ObjectMeta,
    };

    use super::*;
    use std::time::Duration;

    #[tokio::test]
    #[ignore = "requires a k8s cluster"]
    async fn can_create_data_set_task_and_finalize_data_set() -> Result<()> {
        let client = Client::try_default().await?;
        let namespace = "default".to_string();
        let _tasks = Api::<Task>::namespaced(client.clone(), &namespace);
        let data_sets = Api::<DataSet>::namespaced(client.clone(), &namespace);
        let context = super::Context {
            client: client.clone(),
            namespace,
        };

        let data_set = DataSet {
            metadata: ObjectMeta {
                generate_name: Some("testdataset2".to_string()),
                finalizers: Some(vec![super::DATA_SET_CONTROLLER.to_string()]),
                ..ObjectMeta::default()
            },
            spec: DataSetSpec {
                cfg: DataSetCfg {
                    name: "test".to_string(),
                    path: "data".to_string(),
                    task: Some(TaskCfg {
                        name: Some("testname".to_string()),
                        task_ref: Some(TaskRef {
                            name: "test_data_task".to_string(),
                            project: None,
                        }),
                    }),
                    size: None,
                },
                deletion_approved: false,
            },
            status: Some(DataSetStatus {
                phase: DataSetPhase::Pending {},
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
