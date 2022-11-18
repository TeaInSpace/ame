// WARNING: generated by kopium - manual changes will be overwritten
use k8s_openapi::api::core::v1::{
    Container, LocalObjectReference, PersistentVolumeClaim, PodSecurityContext, Volume,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_tuple::*;
use std::collections::BTreeMap;
use std::default::Default;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "Workflow",
    group = "argoproj.io",
    version = "v1alpha1",
    namespaced
)]
#[kube(status = "WorkflowStatus", shortname = "wf")]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSpec {
    pub entrypoint: String,
    pub templates: Option<Vec<WorkflowTemplate>>,
    pub imagepullsecrets: Option<Vec<LocalObjectReference>>,
    pub volume_claim_templates: Option<Vec<PersistentVolumeClaim>>,
    pub volumes: Option<Vec<Volume>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct WorkflowStatus {
    pub phase: WorkflowPhase,
}

// TODO: How do we handle WorkflowPhase unknown
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum WorkflowPhase {
    Pending,
    Running,
    Succeeded,
    Failed,
    Error,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct WorkflowTemplate {
    pub name: String,
    pub metadata: Option<PodMetadata>,
    pub steps: Option<Vec<Vec<WorkflowStep>>>,
    pub securitycontext: Option<PodSecurityContext>,
    pub script: Option<ArgoScriptTemplate>,
    pub podspecpatch: Option<String>,
}

impl WorkflowTemplate {
    pub fn new(name: String) -> WorkflowTemplate {
        WorkflowTemplate {
            name,
            metadata: None,
            steps: None,
            securitycontext: None,
            script: None,
            podspecpatch: None,
        }
    }

    pub fn add_parallel_step(&mut self, steps: Vec<WorkflowStep>) -> &mut WorkflowTemplate {
        match &mut self.steps {
            Some(psteps) => &psteps.push(steps),
            None => &(self.steps = Some(vec![steps])),
        };

        self
    }

    pub fn label(&mut self, key: String, val: String) -> &mut WorkflowTemplate {
        match &mut self.metadata {
            Some(m) => match &mut m.labels {
                Some(labels) => {
                    labels.insert(key, val);
                }
                None => {
                    let mut labels = BTreeMap::new();
                    labels.insert(key, val);
                }
            },
            None => {
                let mut labels = BTreeMap::new();
                labels.insert(key, val);

                self.metadata = Some(PodMetadata {
                    labels: Some(labels),
                    annotations: None,
                })
            }
        }

        self
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct ArgoScriptTemplate {
    #[serde(flatten)]
    pub container: Container,

    pub source: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct PodMetadata {
    pub labels: Option<BTreeMap<String, String>>,
    pub annotations: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, JsonSchema, Serialize_tuple, Deserialize_tuple)]
pub struct ParallelSteps {
    pub steps: Vec<WorkflowStep>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct WorkflowStep {
    pub name: String,
    pub inline: Option<Box<WorkflowTemplate>>,
}

impl Default for Workflow {
    fn default() -> Self {
        Workflow {
            metadata: ObjectMeta::default(),
            spec: WorkflowSpec {
                entrypoint: "main".to_string(),
                ..WorkflowSpec::default()
            },
            status: None,
        }
    }
}

impl Workflow {
    pub fn add_pull_secret(&mut self, name: String) -> &mut Workflow {
        let mut refs = if let Some(obj_refs) = self.spec.imagepullsecrets.clone() {
            obj_refs
        } else {
            vec![]
        };

        refs.push(LocalObjectReference { name: Some(name) });

        self.spec.imagepullsecrets = Some(refs);
        self
    }

    pub fn gen_name(&mut self, name: String) -> &mut Workflow {
        self.metadata.generate_name = Some(name);
        self
    }

    pub fn set_name(&mut self, name: String) -> &mut Workflow {
        self.metadata.name = Some(name);
        self
    }

    pub fn label(&mut self, key: String, val: String) -> &mut Workflow {
        let mut labels = if let Some(labels) = self.metadata.labels.clone() {
            labels
        } else {
            BTreeMap::new()
        };

        labels.insert(key, val);

        self.metadata.labels = Some(labels);
        self
    }

    pub fn add_template(&mut self, template: WorkflowTemplate) -> &mut Workflow {
        let mut templates = if let Some(templates) = self.spec.templates.clone() {
            templates
        } else {
            vec![]
        };

        templates.push(template);

        self.spec.templates = Some(templates);
        self
    }

    pub fn add_volume_claim_template(&mut self, template: PersistentVolumeClaim) -> &mut Workflow {
        match &mut self.spec.volume_claim_templates {
            Some(templates) => templates.push(template),
            None => self.spec.volume_claim_templates = Some(vec![template]),
        };

        self
    }

    pub fn add_volume(&mut self, volume: Volume) -> &mut Workflow {
        match &mut self.spec.volumes {
            Some(volumes) => volumes.push(volume),
            None => self.spec.volumes = Some(vec![volume]),
        };

        self
    }

    pub fn add_owner_reference(&mut self, owner_reference: OwnerReference) -> &mut Workflow {
        match &mut self.metadata.owner_references {
            Some(refs) => refs.push(owner_reference),
            None => self.metadata.owner_references = Some(vec![owner_reference]),
        };

        self
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn can_add_pull_secret() {
        let mut wf = Workflow::default();

        assert_eq!(wf.spec.imagepullsecrets, None);

        let secret_name = "mysecret".to_string();
        wf.add_pull_secret(secret_name.clone());

        match wf.spec.imagepullsecrets {
            Some(ref secrets) => {
                assert_eq!(secrets.len(), 1);
                assert_eq!(secrets[0].name.as_ref().unwrap(), &secret_name);
            }
            None => panic!("failed to add pull secret"),
        }

        let second_secret_name = "mysecret2".to_string();
        wf.add_pull_secret(second_secret_name.clone());
        match wf.spec.imagepullsecrets {
            Some(secrets) => {
                assert_eq!(secrets.len(), 2);
                assert_eq!(secrets[1].name.as_ref().unwrap(), &second_secret_name);
            }
            None => panic!("failed to add pull secret"),
        }
    }
}