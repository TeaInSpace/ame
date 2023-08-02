use crate::{custom_resources::new_task::Task, grpc::TaskCfg, AmeError, Result};

use kube::{core::ObjectMeta, Client, CustomResource, Resource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::grpc::{task_status::Phase, DataSetCfg};

use super::new_task::{ProjectSource, TaskSpec};

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
    pub project: Option<String>,
}

#[derive(Debug, Clone, JsonSchema, Serialize, Deserialize, Default)]
pub struct DataSetStatus {
    pub phase: Option<DataSetPhase>,
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

        // TODO: implement default trait for task phase.
        let task_phase = task.status.unwrap_or_default().phase.unwrap_or(
            crate::grpc::task_status::Phase::Pending(crate::grpc::TaskPhasePending {}),
        );

        match task_phase {
            Phase::Running(_) | Phase::Pending(_) => DataSetPhase::RunningTask { task_name },
            Phase::Succeeded(_) => DataSetPhase::Ready { task_name },
            Phase::Failed(_) => DataSetPhase::Failed { task_name },
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
    pub fn from_cfg(name: &str, cfg: DataSetCfg) -> Self {
        Self::new(
            name,
            DataSetSpec {
                cfg,
                deletion_approved: false,
                project: None,
            },
        )
    }

    pub async fn running_task(&self, _client: Client) -> Option<String> {
        self.status.as_ref().and_then(|s| match &s.phase {
            Some(DataSetPhase::RunningTask { task_name }) => Some(task_name.clone()),
            _ => None,
        })
    }

    pub fn phase(&self) -> &DataSetPhase {
        self.status
            .as_ref()
            .map(|s| &s.phase)
            .unwrap_or(&Some(DataSetPhase::Pending {}))
            .as_ref()
            .unwrap_or_default()
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
        let name = task_cfg
            .name
            .as_ref()
            .unwrap_or(&default_name)
            .replace('_', "-"); // TODO sanitize names

        let mut spec = TaskSpec::from(task_cfg.clone());

        spec.project = self.spec.project.clone();
        if let Some(repo) = self.annotations().get("gitrepository") {
            spec.source = Some(ProjectSource::from_public_git_repo(repo.to_string()));
        }

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
