use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::chrono::OutOfRangeError;
use k8s_openapi::chrono::ParseError;

use kube::ResourceExt;
use secrets::SecretError;
use std::env::VarError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Finalizer Error: {0}")]
    FinalizerError(#[source] kube::runtime::finalizer::Error<kube::Error>),

    #[error("SerializationError: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Received error from kube API: {0}")]
    KubeApiError(#[from] kube::Error),

    #[error("Failed to find projet source: {0}")]
    MissingProjectSrc(String),

    #[error("libgit2 produced an error: {0}")]
    GitError(#[from] git2::Error),

    #[error("Ame errored: {0}")]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("Ame errored: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Ame erroed: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),

    #[error("Ame errored: {0}")]
    K8sTimeError(#[from] OutOfRangeError),

    #[error("Invalid project source: {0}")]
    InvalidProjectSrc(String),

    #[error("Environment variable was not present: {0}")]
    MissingEnvVariable(#[from] VarError),

    #[error("No model deployment was found")]
    MissingDeployment(),

    #[error("No matching template found was found: {0} {1}")]
    MissingTemplate(String, String),

    #[error("No Mlfow URL was found")]
    MissingMlflowUrl(),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Failed to merge structs: {0}")]
    MergeError(#[from] serde_merge::error::Error),

    #[error("failed to pass duration: {0}")]
    InvalidDuration(String),

    #[error("got error while converting: {0}")]
    ConversionError(String),

    #[error("got error while passing time: {0}")]
    ChronoParseError(#[from] ParseError),

    #[error("got error from AME's secret store: {0}")]
    SecretError(#[from] SecretError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub mod manager;
pub use manager::Task;
pub use manager::TaskSpec;
pub use manager::TaskType;

pub mod argo;
pub use argo::Workflow;
pub use argo::WorkflowPhase;

pub mod project;
pub use project::*;

pub mod project_source;
pub use project_source::ProjectSource;
pub use project_source::ProjectSourceSpec;

pub mod common;
pub mod project_source_ctrl;
pub mod secrets;

use ame::grpc::*;

impl TryFrom<self::CreateTaskRequest> for Task {
    type Error = Error;

    fn try_from(t: CreateTaskRequest) -> Result<Self> {
        let CreateTaskRequest {
            id: Some(TaskIdentifier { name: id }),
            template: Some(template),
        } = t else {
            return Err(Error::ConversionError("Failed to extract id and template from CreateTaskRequest".to_string()))
        };

        Ok(Task {
            metadata: ObjectMeta {
                name: Some(id),
                ..ObjectMeta::default()
            },
            spec: TaskSpec {
                projectid: Some(template.projectid),
                runcommand: Some(template.command),
                image: template.image,
                task_type: template.task_type.map(|t| {
                    if t == 1 {
                        TaskType::Mlflow
                    } else {
                        TaskType::PipEnv
                    }
                }),
                ..TaskSpec::default()
            },
            status: None,
        })
    }
}

impl From<ProjectSourceCfg> for ProjectSource {
    fn from(project_src: ProjectSourceCfg) -> Self {
        ProjectSource {
            metadata: ObjectMeta {
                generate_name: Some("ameprojectsrc".to_string()),
                ..ObjectMeta::default()
            },
            spec: ProjectSourceSpec { cfg: project_src },
            status: None,
        }
    }
}

impl From<TaskTemplate> for Task {
    fn from(t: TaskTemplate) -> Self {
        Task {
            metadata: ObjectMeta {
                generate_name: Some("mytask".to_string()),
                ..ObjectMeta::default()
            },
            spec: TaskSpec {
                projectid: Some(t.projectid),
                runcommand: Some(t.command),
                image: t.image,
                task_type: t.task_type.map(|t| {
                    if t == 1 {
                        TaskType::Mlflow
                    } else {
                        TaskType::PipEnv
                    }
                }),
                ..TaskSpec::default()
            },
            status: None,
        }
    }
}

impl From<Task> for TaskTemplate {
    fn from(t: Task) -> Self {
        TaskTemplate {
            name: "".to_string(),
            command: t.spec.runcommand.unwrap_or("".to_string()),
            projectid: t.spec.projectid.unwrap_or("".to_string()),
            image: t.spec.image,
            task_type: t
                .spec
                .task_type
                .map(|t| if t == TaskType::Mlflow { 1 } else { 0 }),
        }
    }
}

impl From<Task> for TaskIdentifier {
    fn from(t: Task) -> Self {
        TaskIdentifier { name: t.name_any() }
    }
}
