use crate::grpc::TaskCfg;
use crate::AmeError;
use crate::{custom_resources::task::Task, Result};

use kube::core::ObjectMeta;
use kube::{Client, CustomResource, Resource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grpc::DataSetCfg;

use super::task::{TaskPhase, TaskSpec};

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
    pub deletion_approved: bool,
}

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, Default)]
pub struct DataSetStatus {
    pub phase: DataSetPhase,
}

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataSetPhase {
    Pending {},
    RunningTask { task_name: String },
    Ready { task_name: String },
    Failed { task_name: String },
}

impl DataSetPhase {
    pub fn from_task(task: Task) -> DataSetPhase {
        let task_name = task.name_any();

        let task_phase = task.status.unwrap_or_default().phase.unwrap_or_default();

        match task_phase {
            TaskPhase::Running | TaskPhase::Error | TaskPhase::Pending => {
                DataSetPhase::RunningTask { task_name }
            }
            TaskPhase::Succeeded => DataSetPhase::Ready { task_name },
            TaskPhase::Failed => DataSetPhase::Failed { task_name },
        }
    }
}

impl Default for DataSetPhase {
    fn default() -> Self {
        DataSetPhase::Pending {}
    }
}

impl Default for &DataSetPhase {
    fn default() -> Self {
        &DataSetPhase::Pending {}
    }
}

impl DataSet {
    pub async fn running_task(&self, _client: Client) -> Option<String> {
        self.status.as_ref().and_then(|s| match &s.phase {
            DataSetPhase::RunningTask { task_name } => Some(task_name.clone()),
            _ => None,
        })
    }

    pub fn phase(&self) -> &DataSetPhase {
        self.status.as_ref().map(|s| &s.phase).unwrap_or_default()
    }

    pub fn task_cfg(&self) -> &Option<TaskCfg> {
        &self.spec.cfg.task
    }

    pub fn generate_task(&self) -> Result<Task> {
        let Some(owner_ref) = self.controller_owner_ref(&()) else {
        return Err(AmeError::MissingOwnerRef(self.name_any()));
    };

        let Some(ref task_cfg) = self.spec.cfg.task else {
        return Err(AmeError::MissingTaskCfg(self.name_any()));
    };

        // TODO: this should be removed once task_cfg is fleshed out.
        let default_name = "datatask".to_string();
        let name = task_cfg.name.as_ref().unwrap_or(&default_name);

        let spec = TaskSpec::try_from(task_cfg.clone())?;

        let metadata = ObjectMeta {
            name: Some(format!("{}{}", self.name_any(), name)),
            owner_references: Some(vec![owner_ref]),
            ..Default::default()
        };

        Ok(Task {
            metadata,
            spec,
            status: None,
        })
    }
}
