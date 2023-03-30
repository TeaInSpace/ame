#[cfg(feature = "native-client")]
use http::uri::InvalidUri;
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
