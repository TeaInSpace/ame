use std::fmt::Debug;

use crate::{
    custom_resources::common::kind_is_project,
    error::AmeError,
    grpc::{resource_id, ProjectSourceId, ResourceId},
    Result,
};
use async_trait::async_trait;
use futures::StreamExt;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::{
    api::{DeleteParams, ListParams, Patch, PatchParams, PostParams},
    runtime::{watcher, WatchStreamExt},
    Api, CustomResourceExt, Resource, ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_merge::omerge;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;

fn oref_is_project(oref: &OwnerReference) -> bool {
    kind_is_project(&oref.kind)
}

impl<K: Resource<DynamicType = ()> + CustomResourceExt> AmeResource for K {
    fn gen_owner_ref(&self) -> Option<OwnerReference> {
        let mut oref = self.controller_owner_ref(&())?;
        oref.controller = Some(false);
        Some(oref)
    }
}

pub trait AmeResource: ResourceExt {
    fn parent_project_oref(&self) -> Result<OwnerReference> {
        let potential_orefs: Vec<OwnerReference> = self
            .meta()
            .owner_references
            .to_owned()
            .map(|orefs| orefs.into_iter().filter(oref_is_project).collect())
            .unwrap_or(vec![]);
        if potential_orefs.len() != 1 {
            return Err(AmeError::FailedToFindParentProjectOref(
                self.name_any(),
                potential_orefs.len(),
            ));
        }

        Ok(potential_orefs[0].clone())
    }

    fn parent_project_name(&self) -> Result<String> {
        Ok(self.parent_project_oref()?.name)
    }

    fn gen_owner_ref(&self) -> Option<OwnerReference>;
}

#[async_trait]
pub trait AmeKubeResourceCtrl {
    type KubeResource: Resource
        + Clone
        + Serialize
        + DeserializeOwned
        + Send
        + Debug
        + 'static
        + Sync
        + ResourceExt;
    type ResourceStatus: TryFrom<Self::KubeResource, Error = AmeError> + Debug + Send + 'static;
    type ResourceId: From<Self::KubeResource>
        + Send
        + Into<ListParams>
        + KubeName
        + Sync
        + From<String>;
    type ResourceCfg: TryInto<Self::KubeResource>
        + Send
        + From<Self::KubeResource>
        + Serialize
        + DeserializeOwned;

    fn api(&self) -> Api<Self::KubeResource>;

    async fn validate_resource(&self, cfg: &Self::KubeResource) -> Result<()>;

    async fn watch(
        &self,
        id: Self::ResourceId,
    ) -> Result<ReceiverStream<std::result::Result<Self::ResourceStatus, tonic::Status>>> {
        let (status_sender, status_receiver) = mpsc::channel(1);
        let kube_resrcs = self.api();
        let list_params = id.into();

        tokio::spawn(async move {
            let status_sender = status_sender.clone();
            let kube_resrcs = kube_resrcs.clone();

            let mut strm = watcher(kube_resrcs, list_params).boxed().applied_objects();

            while let Some(e) = strm.next().await {
                if let Ok(resource) = e {
                    let res = status_sender
                        .send(Self::ResourceStatus::try_from(resource).map_err(Status::from))
                        .await;

                    if res.is_err() {
                        break;
                    }
                }
            }
        });

        Ok(ReceiverStream::new(status_receiver))
    }

    async fn create(
        &self,
        cfg: impl TryInto<Self::KubeResource> + Send,
    ) -> Result<Self::ResourceId> {
        let resource = cfg
            .try_into()
            .map_err(|_e| AmeError::ConversionError("failed to convert".to_string()))?;

        self.validate_resource(&resource).await?;

        Ok(self
            .api()
            .create(&PostParams::default(), &resource)
            .await?
            .into())
    }

    async fn get(&self, id: Self::ResourceId) -> Result<Self::KubeResource> {
        let resource = self.api().get(&id.kube_name()).await?;
        Ok(resource)
    }

    async fn get_status(&self, id: Self::ResourceId) -> Result<Self::ResourceStatus> {
        let status = Self::ResourceStatus::try_from(self.api().get_status(&id.kube_name()).await?)?;
        Ok(status)
    }

    async fn update(&self, id: Self::ResourceId, cfg: Self::ResourceCfg) -> Result<()> {
        let mut resource = self.get(id).await?;
        let name = resource.name_any();
        resource.meta_mut().managed_fields = None;
        let existing_cfg: Self::ResourceCfg = resource.into();

        let new_cfg: Self::ResourceCfg = omerge(existing_cfg, cfg)?;

        let new_resource = new_cfg
            .try_into()
            .map_err(|_e| AmeError::ConversionError("failed to convert".to_string()))?;

        self.api()
            .patch(
                &name,
                &PatchParams::apply("ame").force(),
                &Patch::Apply(new_resource),
            )
            .await?;

        Ok(())
    }

    async fn delete(&self, id: Self::ResourceId) -> Result<()> {
        self.api()
            .delete(&id.kube_name(), &DeleteParams::default())
            .await?;

        Ok(())
    }

    async fn list(&self, params: Option<ListParams>) -> Result<Vec<Self::KubeResource>> {
        Ok(self
            .api()
            .list(&params.unwrap_or(ListParams::default()))
            .await?
            .items)
    }
}

pub trait KubeName {
    fn kube_name(&self) -> String;
}

impl TryFrom<ResourceId> for ListParams {
    type Error = AmeError;

    fn try_from(id: ResourceId) -> Result<Self> {
        match id.id {
            Some(resource_id::Id::ProjectSrcId(ProjectSourceId { name })) => {
                Ok(ListParams::default().fields(&format!("metadata.name=={name}")))
            }
            _ => Err(AmeError::BadResourceId(id)),
        }
    }
}

impl From<ProjectSourceId> for ListParams {
    fn from(id: ProjectSourceId) -> Self {
        ListParams::default().fields(&format!("metadata.name=={}", id.name))
    }
}

impl KubeName for ProjectSourceId {
    fn kube_name(&self) -> String {
        self.name.clone()
    }
}
