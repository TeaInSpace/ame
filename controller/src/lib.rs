use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Finalizer Error: {0}")]
    FinalizerError(#[source] kube::runtime::finalizer::Error<kube::Error>),

    #[error("SerializationError: {0}")]
    SerializationError(#[from] serde_json::Error),

    //TODO: can we have from and source at the same time?
    #[error("Failed to create workflow: {0}")]
    KubeApiError(#[from] kube::Error),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub mod manager;
pub use manager::Task;

pub mod argo;
pub use argo::Workflow;
pub use argo::WorkflowPhase;
