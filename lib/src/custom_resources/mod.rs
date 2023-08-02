use crate::grpc::*;
use k8s_openapi::chrono::{OutOfRangeError, ParseError};
use kube::{api::ListParams, core::ObjectMeta, Api, ResourceExt};
use new_task::Task;
use secrets::SecretError;
use std::env::VarError;
use thiserror::Error;

use self::project_source::{ProjectSource, ProjectSourceSpec};
use project::Project;

pub mod argo;
pub mod common;
pub mod data_set;
pub mod new_task;
pub mod project;
pub mod project_source;
pub mod project_source_ctrl;
pub mod secrets;

pub mod task_ctrl;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Finalizer Error: {0}")]
    FinalizerError(#[source] kube::runtime::finalizer::Error<kube::Error>),

    #[error("SerializationError: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Received error from kube API: {0}")]
    KubeApiError(#[from] kube::Error),

    #[error("Failed to find project source: {0}")]
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

    #[error("failed to find model status for: {0}")]
    MissingModelStatus(String),

    #[error("failed to find validation task status for model: {0}")]
    MissingValidationTask(String),

    #[error("failed to find project id for task: {0}")]
    MissingProjectId(String),

    #[error("failed to find project with id: {0}")]
    MissingProject(String),

    #[error("failed to find data for task with name : {0}")]
    MissingDataSets(String),

    #[error("failed to find AME file project source with name : {0}")]
    MissingAmeFile(String),

    #[error("Task {0} is missing an executor")]
    MissingExecutor(String),

    #[error("Failed to find task cfg {0} referenced in {1}")]
    MissingTaskCfg(String, String),

    #[error("{0}")]
    AmeError(#[from] crate::AmeError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

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

impl From<Task> for TaskIdentifier {
    fn from(t: Task) -> Self {
        TaskIdentifier { name: t.name_any() }
    }
}

pub async fn find_project(
    projects: Api<Project>,
    name: String,
    _source: String,
) -> Result<Project> {
    let matches: Vec<Project> = projects
        .list(&ListParams::default())
        .await?
        .items
        .into_iter()
        .filter(|p| p.spec.cfg.name == name)
        .collect();

    if matches.len() != 1 {
        return Err(Error::MissingProject(name));
    }

    Ok(matches[0].clone())
}
