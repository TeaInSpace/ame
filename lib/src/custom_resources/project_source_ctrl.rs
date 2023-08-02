use crate::{
    ctrl::AmeKubeResourceCtrl,
    custom_resources::project_source::{ProjectSource, ProjectSourceSpec},
    error::AmeError,
    grpc::{ProjectSourceCfg, ProjectSourceId, ProjectSourceStatus},
};
use kube::{
    api::{DeleteParams, ListParams, Patch, PatchParams, PostParams},
    Api, Client, ResourceExt,
};
use serde_merge::omerge;
use thiserror::Error;
use tracing::instrument;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error communicating with Kubernetes: {0}")]
    KubeApi(#[from] kube::Error),

    #[error("received an invalid project source configuration: {0}")]
    InvalidProjectSourceCfg(String),

    #[error("{0}")]
    InternalError(String),

    #[error("failed to find a project source for: {0}")]
    ProjectSourceNotFound(String),

    #[error("failed to merge project source configurations, {0}")]
    FailedProjectSourceCfgMerge(#[from] serde_merge::error::Error),

    #[error("a conflicting project source was found for: {0}")]
    ConflictingProjectSrc(String),
}

type Result<T> = std::result::Result<T, Error>;

impl From<Error> for tonic::Status {
    fn from(error: Error) -> Self {
        Self::from_error(Box::new(error))
    }
}

impl From<ProjectSource> for ProjectSourceId {
    fn from(project_src: ProjectSource) -> Self {
        ProjectSourceId {
            name: project_src.name_any(),
        }
    }
}

#[derive(Debug)]
pub struct ProjectSrcCtrl {
    project_srcs: Api<ProjectSource>,
}

impl ProjectSrcCtrl {
    pub fn new(client_cfg: Client, ns: &str) -> Self {
        Self {
            project_srcs: Api::namespaced(client_cfg, ns),
        }
    }

    pub async fn try_namespaced(ns: &str) -> Result<Self> {
        let client_cfg = Client::try_default().await?;

        Ok(Self {
            project_srcs: Api::namespaced(client_cfg, ns),
        })
    }

    pub async fn create_project_src(&self, cfg: &ProjectSourceCfg) -> Result<ProjectSourceId> {
        let Some(repository) = cfg.git_repository() else {
            return Err(Error::InvalidProjectSourceCfg(
                "missing git configuration".to_string(),
            ));
        };

        match self.get_project_src_for_repo(repository).await {
            Err(Error::ProjectSourceNotFound(_)) => (),
            _ => {
                return Err(Error::ConflictingProjectSrc(repository.to_string()));
            }
        };

        let project_src = self
            .project_srcs
            .create(&PostParams::default(), &ProjectSource::from(cfg.to_owned()))
            .await?;

        Ok(project_src.into())
    }

    pub async fn update_project_src(&self, cfg: &ProjectSourceCfg) -> Result<ProjectSourceId> {
        let Some(repository) = cfg.git_repository() else {
            return Err(Error::InvalidProjectSourceCfg(
                "missing git configuration".to_string(),
            ));
        };

        let mut project_src = self.get_project_src_for_repo(repository).await?;

        project_src.metadata.managed_fields = None;

        let new_cfg = omerge(project_src.clone().spec.cfg, cfg)?;

        let patched_project_src = ProjectSource {
            spec: ProjectSourceSpec { cfg: new_cfg },
            ..project_src.clone()
        };

        self.project_srcs
            .patch(
                &project_src.name_any(),
                &PatchParams::apply("ame").force(),
                &Patch::Apply(patched_project_src),
            )
            .await?;

        Ok(ProjectSourceId {
            name: project_src.name_any(),
        })
    }

    pub async fn get_project_src_for_repo(&self, repository: &str) -> Result<ProjectSource> {
        let potential_project_srcs: Vec<ProjectSource> = self
            .project_srcs
            .list(&ListParams::default())
            .await?
            .items
            .into_iter()
            .filter(|ps| {
                if let Some(ref git_src) = ps.spec.cfg.git {
                    git_src.repository == repository
                } else {
                    false
                }
            })
            .collect();

        if potential_project_srcs.len() > 1 {
            return Err(Error::InternalError(format!("multiple project sources were found for the same repository: {repository}, this should not be possible")));
        }

        if potential_project_srcs.is_empty() {
            return Err(Error::ProjectSourceNotFound(repository.to_string()));
        }

        Ok(potential_project_srcs[0].clone())
    }

    pub async fn delete_project_src_for_repo(&self, repository: &str) -> Result<()> {
        let project_src = self.get_project_src_for_repo(repository).await?;

        self.project_srcs
            .delete(&project_src.name_any(), &DeleteParams::default())
            .await?;

        Ok(())
    }

    #[instrument]
    pub async fn list_project_src(&self) -> Result<Vec<ProjectSourceId>> {
        Ok(self
            .project_srcs
            .list(&ListParams::default())
            .await?
            .into_iter()
            .map(|s| s.name_any().into())
            .collect())
    }
}

#[async_trait::async_trait]
impl AmeKubeResourceCtrl for ProjectSrcCtrl {
    type KubeResource = ProjectSource;
    type ResourceStatus = ProjectSourceStatus;
    type ResourceId = ProjectSourceId;
    type ResourceCfg = ProjectSourceCfg;

    fn api(&self) -> Api<Self::KubeResource> {
        self.project_srcs.clone()
    }

    async fn validate_resource(&self, cfg: &Self::KubeResource) -> crate::Result<()> {
        let Some(repository) = cfg.spec.cfg.git_repository() else {
            return Err(AmeError::InvalidProjectSourceCfg(
                "missing git configuration".to_string(),
            ));
        };

        match self.get_project_src_for_repo(repository).await {
            Err(Error::ProjectSourceNotFound(_)) => (),
            _ => {
                return Err(AmeError::ConflictingProjectSrc(repository.to_string()));
            }
        };

        Ok(())
    }
}

impl TryFrom<ProjectSource> for ProjectSourceStatus {
    type Error = AmeError;

    fn try_from(ps: ProjectSource) -> crate::Result<Self> {
        ps.clone().status.ok_or(AmeError::ConversionError(format!(
            "missing status field in project source {}",
            ps.name_any()
        )))
    }
}

impl From<ProjectSource> for ProjectSourceCfg {
    fn from(ps: ProjectSource) -> Self {
        ps.spec.cfg
    }
}
impl From<Api<ProjectSource>> for ProjectSrcCtrl {
    fn from(api: Api<ProjectSource>) -> Self {
        Self { project_srcs: api }
    }
}
