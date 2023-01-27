use k8s_openapi::chrono::OutOfRangeError;
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

    #[error("libgit2 produce an error: {0}")]
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

    #[error("Failed to get secret object in cluster: {0}")]
    MissingSecret(String),

    #[error("Failed failed to extract secret from secret object: {0}")]
    MissingSecretKey(String),

    #[error("Environment variable was not present: {0}")]
    MissingEnvVariable(#[from] VarError),

    #[error("No model deployment was found")]
    MissingDeployment(),

    #[error("No Mlfow URL was found")]
    MissingMlflowUrl(),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
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
pub mod project_source;
pub use project_source::GitProjectSource;
pub use project_source::ProjectSource;
pub use project_source::ProjectSourceSpec;
