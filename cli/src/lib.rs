pub mod project;

use ame::{
    client::native_client::{build_ame_client, AmeClient},
    AmeServiceClientCfg,
};
use envconfig::Envconfig;

use http::uri::InvalidUri;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ame errored: {0}")]
    CliError(String),

    #[error("Ame errored: {0}")]
    FileError(#[from] std::io::Error),

    #[error("The AME server could not be reached: {0}")]
    TonicError(#[from] tonic::transport::Error),

    #[error("The AME sever failed a request: {0}")]
    TonicStatusError(#[from] tonic::Status),

    #[error("Ame errored: {0}")]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("Failed to extract value from project file: {0}")]
    EmptyProjectField(String),

    #[error("The project file is in a bad state: {0}")]
    MissConfiguredProject(String),

    #[error("Could not find a task with the given name: {0} in the project file")]
    MissingTaskTemplate(String),

    #[error("A project file already exists")]
    ProjectAlreadyExists(),

    #[error("failed to construct configuration: {0}")]
    ConfigError(#[from] confy::ConfyError),

    #[error("got filesystem related error: {0}")]
    WalkDir(#[from] walkdir::Error),

    #[error("failed to parse URL: {0}")]
    ParseError(#[from] ParseError),

    #[error("Ame errored: {0}")]
    ClientError(#[from] ame::error::AmeError),

    #[error("Invalid URI: {0}")]
    UriParseError(#[from] InvalidUri),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub mod project_cmd;
pub mod projectsrc;
pub mod secrets;
pub mod task;

#[derive(Clone, Default, Deserialize, Serialize, Envconfig, PartialEq, Debug)]
pub struct CliConfiguration {
    #[envconfig(from = "AME_ENDPOINT")]
    pub endpoint: String,
    pub id_token: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl CliConfiguration {
    /// This method gathers configuration from a configuration file and environment.
    ///
    /// Configuration option set in the environment take precedent over options
    /// in the configuration file.
    pub fn gather() -> Result<Self> {
        let local_config = confy::load("ame", None);
        let env_config = Envconfig::init_from_env();

        match (local_config, env_config) {
            (_, Ok(env)) => Ok(env),
            (Ok(local), _) => Ok(local),
            (Err(local_err), Err(_)) => Err(Error::ConfigError(local_err)),
        }
    }

    pub fn save(&self) -> Result<()> {
        Ok(confy::store("ame", None, self)?)
    }

    pub fn set_auth_details(
        &mut self,
        id_token: String,
        access_token: String,
        refresh_token: String,
    ) {
        self.id_token = Some(id_token);
        self.access_token = Some(access_token);
        self.refresh_token = Some(refresh_token);
    }

    pub fn init_with_endpoint(endpoint: String) -> Self {
        CliConfiguration {
            endpoint,
            ..CliConfiguration::default()
        }
    }

    pub async fn ame_client(&self) -> Result<AmeClient> {
        Ok(build_ame_client(AmeServiceClientCfg {
            disable_tls_cert_check: true, // TODO: the CLI needs some configuration for this.
            endpoint: self.endpoint.parse().unwrap(),
            id_token: self.id_token.clone(),
        })
        .await?)
    }
}

impl TryFrom<CliConfiguration> for AmeServiceClientCfg {
    type Error = Error;
    fn try_from(cli_cfg: CliConfiguration) -> std::result::Result<Self, Self::Error> {
        Ok(AmeServiceClientCfg {
            disable_tls_cert_check: true,
            endpoint: cli_cfg.endpoint,
            id_token: cli_cfg.id_token,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_cli_configuration_from_env() -> Result<(), Box<dyn std::error::Error>> {
        let temp = assert_fs::TempDir::new()?;
        std::env::set_var("XDG_CONFIG_HOME", temp.to_str().unwrap());

        let correct_endpoint = "someendpoint";
        std::env::set_var("AME_ENDPOINT", correct_endpoint);
        let config = CliConfiguration::gather()?;

        assert_eq!(correct_endpoint, config.endpoint);
        std::env::remove_var("AME_ENDPOINT");
        Ok(())
    }

    #[test]
    #[serial]
    fn test_cli_configuration_from_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = assert_fs::TempDir::new()?;
        std::env::set_var("XDG_CONFIG_HOME", temp.to_str().unwrap());

        let correct_config = CliConfiguration {
            endpoint: "anendpoint".to_string(),
            id_token: Some("an id token".to_string()),
            refresh_token: Some("a refresh token".to_string()),
            access_token: Some("an access token".to_string()),
        };

        correct_config.save()?;

        similar_asserts::assert_eq!(correct_config, CliConfiguration::gather()?);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_cli_configuration_override_file() -> Result<(), Box<dyn std::error::Error>> {
        let temp = assert_fs::TempDir::new()?;
        std::env::set_var("XDG_CONFIG_HOME", temp.to_str().unwrap());

        let file_config = CliConfiguration {
            endpoint: "anendpoint".to_string(),
            id_token: Some("an id token".to_string()),
            refresh_token: Some("a refresh token".to_string()),
            access_token: Some("an access token".to_string()),
        };

        file_config.save()?;

        let correct_endpoint = "correctendpoint";
        std::env::set_var("AME_ENDPOINT", correct_endpoint);

        let config = CliConfiguration::gather()?;

        similar_asserts::assert_eq!(config.endpoint, correct_endpoint);

        std::env::remove_var("AME_ENDPOINT");
        Ok(())
    }
}
