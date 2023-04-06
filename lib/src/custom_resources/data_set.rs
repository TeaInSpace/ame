use crate::grpc::TaskCfg;

use kube::{Client, CustomResource};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grpc::DataSetCfg;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "DataSet",
    group = "ame.teainspace.com",
    version = "v1alpha1",
    namespaced
)]
#[kube(status = "DataSetStatus", shortname = "ds")]
#[serde(rename_all = "camelCase")]
pub struct DataSetSpec {
    #[serde(flatten)]
    pub cfg: DataSetCfg,
}

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, Default)]
pub struct DataSetStatus {
    pub phase: Phase,
}

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Pending {},
    RunningTask { task_name: String },
    Ready { task_name: String },
    Failed { task_name: String },
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Pending {}
    }
}

impl Default for &Phase {
    fn default() -> Self {
        &Phase::Pending {}
    }
}

impl DataSet {
    pub async fn running_task(&self, _client: Client) -> Option<String> {
        self.status.as_ref().and_then(|s| match &s.phase {
            Phase::RunningTask { task_name } => Some(task_name.clone()),
            _ => None,
        })
    }

    pub fn phase(&self) -> &Phase {
        self.status.as_ref().map(|s| &s.phase).unwrap_or_default()
    }

    pub fn task_spec(&self) -> &Option<TaskCfg> {
        &self.spec.cfg.task
    }
}
