#[cfg(feature = "native-client")]
use http::uri::InvalidUri;

#[cfg(feature = "custom-resources")]
use kube::runtime::finalizer;

use thiserror::Error;
use tonic::Status;

#[cfg(feature = "native-client")]
use url::ParseError;

use crate::grpc::ResourceId;

#[derive(Error, Debug)]
pub enum AmeError {
    #[error("Got error from API: {0}")]
    ApiError(String),

    #[error("Got an invalid resource ID: {0}")]
    BadResourceId(ResourceId),

    #[cfg(feature = "ame-control")]
    #[error("error communicating with Kubernetes: {0}")]
    KubeApi(#[from] kube::Error),

    #[cfg(feature = "ame-control")]
    #[error("failed to merge: {0}")]
    MergeError(#[from] serde_merge::error::Error),

    #[error("failed to convert: {0}")]
    ConversionError(String),

    #[error("project source config is invalid: {0}")]
    InvalidProjectSourceCfg(String),

    #[error("found a project source with a conflicting repository: {0}")]
    ConflictingProjectSrc(String),

    #[error("missing parameter from request: {0}")]
    MissingRequestParameter(String),

    #[cfg(feature = "native-client")]
    #[error("failed to parse endpoint")]
    ParsingFailure,

    #[cfg(feature = "native-client")]
    #[error("{0}")]
    InvalidUri(#[from] InvalidUri),

    #[cfg(feature = "native-client")]
    #[error("{0}")]
    IoError(#[from] std::io::Error),

    #[cfg(feature = "native-client")]
    #[error("{0}")]
    ParseError(#[from] ParseError),

    #[cfg(feature = "native-client")]
    #[error("{0}")]
    AuthError(String),

    #[error("finalizer failed: {0}")]
    FinalizerError(String),

    // TODO: this error needs to be made more useful.
    #[error("missing task config for resource: {0}")]
    MissingTaskCfg(String),

    #[error("failed to find a task ref for: {0}")]
    MissingTaskRef(String),

    #[error("failed to get owner reference for resource: {0}")]
    MissingOwnerRef(String),

    #[error("missing status field for: {0}")]
    MissingTaskStatus(String),
}

impl From<Status> for AmeError {
    fn from(s: Status) -> Self {
        AmeError::ApiError(s.message().to_string())
    }
}

impl From<AmeError> for tonic::Status {
    fn from(error: AmeError) -> Self {
        Self::from_error(Box::new(error))
    }
}

#[cfg(feature = "custom-resources")]
impl<T: std::error::Error> From<finalizer::Error<T>> for AmeError {
    fn from(error: finalizer::Error<T>) -> Self {
        // TODO: how do we handle this error conversion properly?
        AmeError::FinalizerError(error.to_string())
    }
}
