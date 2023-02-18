use url::ParseError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ame errored: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Serde errored: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("Openid errored: {0}")]
    OpenIdError(String),

    #[error("Failed to parse metadata: {0}")]
    MedatadataError(#[from] InvalidMetadataValue),

    #[error("failed to parse URL: {0}")]
    ParseError(#[from] ParseError),

    #[error("Authentication failed: {0}")]
    AuthError(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub use proto::*;

use tonic::metadata::errors::InvalidMetadataValue;

pub mod auth;
pub mod client_builder;

#[cfg(test)]
mod tests {}
