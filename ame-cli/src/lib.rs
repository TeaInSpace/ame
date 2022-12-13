pub mod project;

use envconfig::Envconfig;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Ame errored: {0}")]
    CliError(String),

    #[error("Ame errored: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Ame errored: {0}")]
    TonicError(#[from] tonic::transport::Error),

    #[error("Ame errored: {0}")]
    TonicStatusError(#[from] tonic::Status),

    #[error("Ame errored: {0}")]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("Failed to extract value from project file: {0}")]
    EmptyProjectField(String),

    #[error("The project file is in a bad state: {0}")]
    MisConfiguredProject(String),

    #[error("Could not find a task with the given name: {0} in the project file")]
    MissingTaskTemplate(String),

    #[error("A project file already exists")]
    ProjectAlreadyExists(),

    #[error("failed to construct configuration: {0}")]
    ConfigError(#[from] confy::ConfyError),

    #[error("got filesystem related error: {0}")]
    WalkDir(#[from] walkdir::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Default, Deserialize, Serialize, Envconfig, PartialEq, Debug)]
pub struct CliConfiguration {
    #[envconfig(from = "AME_ENDPOINT")]
    pub endpoint: String,
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
