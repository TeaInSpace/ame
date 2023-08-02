use error::AmeError;

#[cfg(feature = "native-client")]
use http::{
    uri::{Authority, Scheme},
    Uri,
};

#[cfg(feature = "custom-resources")]
pub mod custom_resources;

#[cfg(feature = "web-components")]
pub mod web;

#[cfg(any(feature = "web-components", feature = "native-client"))]
pub mod client;

#[cfg(feature = "project-tools")]
pub mod project;

pub mod grpc {
    #![allow(clippy::all)]

    use std::{
        collections::BTreeMap,
        fmt::{self, Display},
    };

    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;

    use self::task_cfg::Executor;

    tonic::include_proto!("ame.v1");

    impl Display for task_status::Phase {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let output = match self {
                task_status::Phase::Pending(_) => "Pending",
                task_status::Phase::Succeeded(_) => "Succeeded",
                task_status::Phase::Running(_) => "Running",
                task_status::Phase::Failed(_) => "Failed",
            };

            write!(f, "{output}")
        }
    }

    impl self::task_status::Phase {
        pub fn success(&self) -> bool {
            if let self::task_status::Phase::Succeeded(_) = self {
                true
            } else {
                false
            }
        }
    }

    impl Display for ResourceId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self.id {
                Some(resource_id::Id::ProjectSrcId(ProjectSourceId { ref name })) => {
                    write!(f, "{name}")
                }
                None => write!(f, "empty ID"),
            }
        }
    }

    impl From<&str> for TaskIdentifier {
        fn from(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    impl From<String> for AmeSecretId {
        fn from(key: String) -> Self {
            AmeSecretId { key }
        }
    }

    impl From<String> for ProjectSourceId {
        fn from(name: String) -> Self {
            Self { name }
        }
    }

    pub fn resource_map_conv(resources: BTreeMap<String, String>) -> BTreeMap<String, Quantity> {
        let mut new_resources = BTreeMap::<String, Quantity>::new();
        for (k, v) in resources.into_iter() {
            new_resources.insert(k, Quantity(v));
        }

        new_resources
    }

    impl TaskLogRequest {
        pub fn stream_from_beginning(task_name: &str, watch: bool) -> Self {
            Self {
                taskid: Some(TaskIdentifier::from(task_name)),
                start_from: Some(1),
                watch: Some(watch),
            }
        }
    }

    impl ProjectSourceCfg {
        pub fn git_repository(&self) -> Option<&str> {
            if let Some(GitProjectSource { ref repository, .. }) = self.git {
                Some(repository)
            } else {
                None
            }
        }

        pub fn new_git_source(
            repository: String,
            username: Option<String>,
            secret: Option<String>,
        ) -> Self {
            Self {
                git: Some(GitProjectSource {
                    repository,
                    username,
                    secret,
                    sync_interval: None,
                }),
            }
        }
        pub fn from_git_repo(repo: String) -> Self {
            Self {
                git: Some(GitProjectSource {
                    repository: repo,
                    ..GitProjectSource::default()
                }),
            }
        }
    }

    impl Executor {
        pub fn command(&self) -> String {
            match self {
                Executor::Poetry(PoetryExecutor {
                    python_version,
                    command,
                }) => {
                    format!(
                        "
                        source ~/.bashrc
                        
                        pyenv install {python_version}

                        pyenv global {python_version}

                        poetry install
                    
                        poetry run {command}
                    "
                    )
                }
                Executor::Mlflow(_) => "export PATH=$HOME/.pyenv/bin:$PATH

             unset AWS_SECRET_ACCESS_KEY

             unset AWS_ACCESS_KEY_ID

             mlflow run ."
                    .to_string(),
                Executor::PipEnv(PipEnvExecutor { command }) => {
                    format!(
                        "
                            pipenv sync

                            pipenv run {command}
                           
                        "
                    )
                }

                Executor::Pip(PipExecutor {
                    python_version,
                    command,
                }) => {
                    format!(
                        "

                        source ~/.bashrc
                        
                        pyenv install {python_version}

                        pyenv global {python_version}

                        pip install -r requirements.txt
                    
                        {command}
                    "
                    )
                }

                Executor::Custom(CustomExecutor {
                    python_version,
                    command,
                }) => {
                    format!(
                        "
                        source ~/.bashrc
                        
                        pyenv install {python_version}

                        pyenv global {python_version}
                    
                        {command}
                    "
                    )
                }
            }
        }
    }
}

pub mod proto;

pub mod api;

pub mod error;

#[cfg(feature = "ame-control")]
pub mod ctrl;

pub type Result<U> = std::result::Result<U, AmeError>;

#[derive(Clone)]
pub struct AmeServiceClientCfg {
    pub disable_tls_cert_check: bool,
    pub endpoint: String,
    pub id_token: Option<String>,
}

#[cfg(feature = "native-client")]
impl AmeServiceClientCfg {
    pub fn scheme(&self) -> Result<Scheme> {
        let uri = self.endpoint.parse::<Uri>()?;
        uri.scheme()
            .map(|s| s.to_owned())
            .ok_or(AmeError::ParsingFailure)
    }

    pub fn authority(&self) -> Result<Authority> {
        let uri = self.endpoint.parse::<Uri>()?;
        uri.authority()
            .map(|a| a.to_owned())
            .ok_or(AmeError::ParsingFailure)
    }
}
