use error::AmeError;

#[cfg(feature = "native-client")]
use http::{
    uri::{Authority, Scheme},
    Uri,
};

//#[cfg(features = "custom-resources")]
pub mod custom_resources;

#[cfg(feature = "web-components")]
pub mod web;

#[cfg(any(feature = "web-components", feature = "native-client"))]
pub mod client;

pub mod grpc {
    #![allow(clippy::all)]

    use std::fmt::{self, Display};

    tonic::include_proto!("ame.v1");

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

    impl TaskLogRequest {
        pub fn stream_from_beginning(task_name: &str, watch: bool) -> Self {
            Self {
                taskid: Some(TaskIdentifier::from(task_name)),
                start_from: Some(1),
                watch: Some(watch),
            }
        }
    }

    impl CreateTaskRequest {
        pub fn new(task_name: &str, task_template: TaskTemplate) -> Self {
            Self {
                id: Some(TaskIdentifier::from(task_name)),
                template: Some(task_template),
            }
        }
    }

    impl TaskProjectDirectoryStructure {
        pub fn new(task_name: &str, project_id: &str, paths: Vec<String>) -> Self {
            Self {
                taskid: Some(TaskIdentifier::from(task_name)),
                projectid: project_id.to_string(),
                paths,
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
