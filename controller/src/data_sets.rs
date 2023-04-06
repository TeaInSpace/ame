use crate::manager::TaskPhase;
use crate::{Task, TaskSpec};
use std::sync::Arc;
use std::time::Duration;

use ame::custom_resources::data_set::{DataSet, DataSetSpec, DataSetStatus, Phase};
use ame::error::AmeError;
use ame::grpc::TaskCfg;
use ame::Result;
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use kube::api::{ListParams, Patch, PatchParams};
use kube::core::ObjectMeta;
use kube::runtime::controller::Action;
use kube::runtime::{finalizer, Controller};
use kube::{Api, Client, Resource, ResourceExt};
use std::default::Default;
use tracing::{error, info};

static DATA_SET_CONTROLLER: &str = "datasets.ame.teainspace.com";

#[derive(Clone)]
struct Context {
    client: Client,
    namespace: String,
}

impl Context {
    fn api(&self) -> (Api<DataSet>, Api<Task>) {
        (
            Api::namespaced(self.client.clone(), &self.namespace),
            Api::namespaced(self.client.clone(), &self.namespace),
        )
    }
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
    let (data_sets, _) = ctx.api();

    info!("reconciling {}", data_set.name_any());

    Ok(
        finalizer(&data_sets, DATA_SET_CONTROLLER, data_set, |event| async {
            match event {
                finalizer::Event::Apply(data_set) => apply(&data_set, ctx).await,
                finalizer::Event::Cleanup(data_set) => cleanup(&data_set, ctx).await,
            }
        })
        .await?,
    )
}

fn error_policy(_src: Arc<DataSet>, error: &AmeError, _ctx: Arc<Context>) -> Action {
    error!("failed to reconcile: {:?}", error);
    Action::requeue(Duration::from_secs(5 * 60))
}

pub async fn run(config: impl Into<DataSetControllerCfg>) -> Result<BoxFuture<'static, ()>> {
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

impl TryFrom<TaskCfg> for TaskSpec {
    type Error = AmeError;
    fn try_from(cfg: TaskCfg) -> Result<Self> {
        Ok(TaskSpec::from_ref(cfg.task_ref.ok_or(
            AmeError::MissingTaskRef(cfg.name.unwrap_or_default()),
        )?))
    }
}

impl From<Task> for Phase {
    fn from(task: Task) -> Self {
        let task_name = task.name_any();

        let task_phase = task.status.unwrap_or_default().phase.unwrap_or_default();

        match task_phase {
            TaskPhase::Running | TaskPhase::Error | TaskPhase::Pending => {
                Phase::RunningTask { task_name }
            }
            TaskPhase::Succeeded => Phase::Ready { task_name },
            TaskPhase::Failed => Phase::Failed { task_name },
        }
    }
}

fn generate_task(data_set: &DataSet) -> Result<Task> {
    let Some(owner_ref) = data_set.controller_owner_ref(&()) else {
        return Err(AmeError::MissingOwnerRef(data_set.name_any()));
    };

    let Some(ref task_cfg) = data_set.spec.cfg.task else {
        return Err(AmeError::MissingTaskCfg(data_set.name_any()));
    };

    let default_name = "datatask".to_string();
    let name = task_cfg.name.as_ref().unwrap_or(&default_name);

    let spec = TaskSpec::try_from(task_cfg.clone())?;

    let metadata = ObjectMeta {
        name: Some(format!("{}{}", data_set.name_any(), name)),
        owner_references: Some(vec![owner_ref]),
        ..Default::default()
    };

    Ok(Task {
        metadata,
        spec,
        status: None,
    })
}

async fn apply(data_set: &DataSet, context: Arc<Context>) -> Result<Action> {
    info!("applying dataset: {}", data_set.name_any());
    let (data_sets, tasks) = context.api();

    let task = generate_task(data_set)?;

    let task = tasks
        .patch(
            &task.name_any(),
            &PatchParams::apply(DATA_SET_CONTROLLER),
            &Patch::Apply(&task),
        )
        .await?;

    let status = DataSetStatus {
        phase: Phase::from(task),
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

async fn cleanup(_data_set: &DataSet, _context: Arc<Context>) -> Result<Action> {
    info!("cleanup");
    Ok(Action::requeue(Duration::from_secs(300)))
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

    #[tokio::test]
    #[ignore = "requires a k8s cluster"]
    async fn can_create_data_set_task() -> Result<()> {
        let client = Client::try_default().await?;
        let namespace = "ame-system".to_string();
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
                },
            },
            status: Some(DataSetStatus {
                phase: Phase::Pending {},
            }),
        };

        let data_set = data_sets.create(&PostParams::default(), &data_set).await?;

        reconcile(Arc::new(data_set.clone()), Arc::new(context.clone()))
            .await
            .unwrap();

        let data_set = data_sets.get(&data_set.name_any()).await?;

        assert!(data_set.running_task(client.clone()).await.is_some());

        data_sets
            .delete(&data_set.name_any(), &DeleteParams::default())
            .await?;

        let data_set = data_sets.get(&data_set.name_any()).await?;
        reconcile(Arc::new(data_set.clone()), Arc::new(context.clone()))
            .await
            .unwrap();

        data_sets.get(&data_set.name_any()).await.unwrap_err();

        Ok(())
    }
}
