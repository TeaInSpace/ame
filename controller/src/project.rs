use crate::TaskSpec;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "Project",
    group = "ame.teainspace.com",
    version = "v1alpha1",
    namespaced,
    status = "ProjectStatus",
    shortname = "proj"
)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSpec {
    #[serde(rename = "projectid")]
    id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    tasks: Option<Vec<TaskSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    templates: Option<Vec<TaskSpec>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Default)]
pub struct ProjectStatus {}

impl Project {
    pub fn add_owner_reference(&mut self, owner_reference: OwnerReference) -> &mut Project {
        match &mut self.metadata.owner_references {
            Some(refs) => refs.push(owner_reference),
            None => self.metadata.owner_references = Some(vec![owner_reference]),
        };

        self
    }
}
