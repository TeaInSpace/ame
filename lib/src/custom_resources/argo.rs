use super::new_task::Task;
use crate::{
    grpc::{secret::Variant, AmeSecretVariant},
    Result,
};
use k8s_openapi::{
    api::core::v1::{
        Container, EnvVar, EnvVarSource, LocalObjectReference, PersistentVolumeClaim,
        PersistentVolumeClaimSpec, PersistentVolumeClaimStatus, PodSecurityContext,
        ResourceRequirements, SecretKeySelector, Volume,
    },
    apimachinery::pkg::{
        api::resource::Quantity,
        apis::meta::v1::{ObjectMeta, OwnerReference},
    },
};
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_tuple::*;
use std::{collections::BTreeMap, default::Default};

use super::new_task::TaskContext;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default, PartialEq)]
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
    pub image_pull_secrets: Option<Vec<LocalObjectReference>>,
    pub volume_claim_templates: Option<Vec<PersistentVolumeClaim>>,
    pub volumes: Option<Vec<Volume>>,
    pub service_account_name: Option<String>,
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

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTemplate {
    pub name: String,
    pub metadata: Option<PodMetadata>,
    pub steps: Option<Vec<Vec<WorkflowStep>>>,
    pub security_context: Option<PodSecurityContext>,
    pub script: Option<ArgoScriptTemplate>,
    pub pod_spec_patch: Option<String>,
}

impl WorkflowTemplate {
    pub fn new(name: String) -> WorkflowTemplate {
        WorkflowTemplate {
            name,
            metadata: Some(PodMetadata::default()),
            steps: None,
            security_context: None,
            script: None,
            pod_spec_patch: None,
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
                    m.labels = Some(labels);
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

    pub fn bulk_annotate(
        &mut self,
        mut new_annos: BTreeMap<String, String>,
    ) -> &mut WorkflowTemplate {
        let mut metadata = if let Some(metadata) = self.metadata.clone() {
            metadata
        } else {
            PodMetadata::default()
        };

        if let Some(mut annotations) = metadata.clone().annotations {
            annotations.append(&mut new_annos);
            metadata.annotations = Some(annotations);
        } else {
            metadata.annotations = Some(new_annos);
        }

        self.metadata = Some(metadata);
        self
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub struct ArgoScriptTemplate {
    #[serde(flatten)]
    pub container: Container,
    pub source: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Default)]
pub struct PodMetadata {
    pub labels: Option<BTreeMap<String, String>>,
    pub annotations: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, JsonSchema, Serialize_tuple, Deserialize_tuple)]
pub struct ParallelSteps {
    pub steps: Vec<WorkflowStep>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub struct WorkflowStep {
    pub name: String,
    pub inline: Option<Box<WorkflowTemplate>>,
}

impl WorkflowStep {
    fn new_inline(name: String, template: WorkflowTemplate) -> Self {
        Self {
            name,
            inline: Some(Box::new(template)),
        }
    }
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
        let mut refs = if let Some(obj_refs) = self.spec.image_pull_secrets.clone() {
            obj_refs
        } else {
            vec![]
        };

        refs.push(LocalObjectReference { name: Some(name) });

        self.spec.image_pull_secrets = Some(refs);
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

    pub fn set_entrypoint(&mut self, template: WorkflowTemplate) -> &mut Workflow {
        self.spec.entrypoint = template.name.clone();
        self.add_template(template);
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

    pub fn set_service_account(&mut self, service_account: String) -> &mut Workflow {
        self.spec.service_account_name = Some(service_account);
        self
    }
}

pub struct WorkflowBuilder {
    templates: Vec<WorkflowTemplate>,
    task_name: String,
    service_account: String,
    owner_reference: Option<OwnerReference>,
    volumes: Vec<(String, BTreeMap<String, Quantity>)>,
}

impl WorkflowBuilder {
    pub fn new(task_name: String, service_account: String) -> Self {
        Self {
            templates: vec![],
            task_name,
            service_account,
            owner_reference: None,
            volumes: vec![],
        }
    }

    pub fn add_owner_reference(&mut self, owner_reference: OwnerReference) -> &mut Self {
        self.owner_reference = Some(owner_reference);
        self
    }

    pub fn add_template(&mut self, template: WorkflowTemplate) -> &mut Self {
        self.templates.push(template);

        self
    }

    pub fn add_volume(&mut self, name: String, resources: BTreeMap<String, Quantity>) -> &mut Self {
        self.volumes.push((name, resources));
        self
    }

    pub fn build(self) -> Result<Workflow> {
        let mut workflow = Workflow::default()
            .set_name(self.task_name.clone())
            .set_service_account(self.service_account)
            .label("ame-task".to_string(), self.task_name.clone())
            .clone();

        let mut main_template = WorkflowTemplate::new("main".to_string());
        for mut template in self.templates {
            template.label("ame-task".to_string(), self.task_name.clone());
            main_template.add_parallel_step(vec![WorkflowStep::new_inline(
                template.name.clone(),
                template,
            )]);
        }

        if let Some(owner_reference) = self.owner_reference {
            workflow.add_owner_reference(owner_reference);
        }

        workflow.set_entrypoint(main_template);
        for (name, resources) in self.volumes {
            workflow.add_volume_claim_template(new_pvc(
                name,
                vec!["ReadWriteOnce".to_string()],
                ResourceRequirements {
                    requests: Some(resources),
                    limits: None,
                },
            ));
        }
        Ok(workflow)
    }
}

fn new_pvc(
    name: String,
    access_mode: Vec<String>,
    resources: ResourceRequirements,
) -> PersistentVolumeClaim {
    PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(name),
            ..ObjectMeta::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            // TODO: make access modes configurable.
            access_modes: Some(access_mode),
            resources: Some(resources),
            ..PersistentVolumeClaimSpec::default()
        }),

        // Note that it is important to create the equivalent of an empty struct here
        // and not just a None.
        // Otherwise the Workflow controller will disagree with AME's controller on
        // how an empty status should be specified.
        status: Some(PersistentVolumeClaimStatus::default()),
    }
}

pub struct WorkflowTemplateBuilder<'a> {
    env: Vec<EnvVar>,
    ctx: &'a TaskContext,
    script: String,
    name: String,
}

impl<'a> WorkflowTemplateBuilder<'a> {
    pub fn new(ctx: &'a TaskContext, script: String, name: String) -> Result<Self> {
        let required_env: Vec<EnvVar> = serde_json::from_value(json!([
            {
            "name":  "AWS_ACCESS_KEY_ID",
            "valueFrom": {
                "secretKeyRef":  {
                    "key": "root-user",
                    "name": "ame-minio",
                    "optional": false,
                }
            },
        },
        {
            "name":  "AWS_SECRET_ACCESS_KEY",
            "valueFrom": {
                "secretKeyRef":  {
                    "key": "root-password",
                    "name": "ame-minio",
                    "optional": false
                }
            },
        },
        {
            "name": "MLFLOW_TRACKING_URI",
            "value": "http://mlflow.ame-system.svc.cluster.local:5000"
        },
        {
            "name":  "MINIO_URL",
            "value": "http://ame-minio.ame-system.svc.cluster.local:9000",
        },

                    {
                        "name":  "PIPENV_YES",
                        "value": "1",
                    },
        ]))?;

        Ok(Self {
            env: required_env,
            ctx,
            script,
            name,
        })
    }

    fn add_env_var(&mut self, var: EnvVar) -> &mut Self {
        self.env.push(var);
        self
    }

    fn add_secret_env_var(&mut self, secret: Variant) -> &mut Self {
        let Variant::Ame(AmeSecretVariant { key, inject_as }) = secret;
        let var = EnvVar {
            name: inject_as,
            value_from: Some(EnvVarSource {
                secret_key_ref: Some(SecretKeySelector {
                    key: "secret".to_string(),
                    name: Some(key),
                    ..SecretKeySelector::default()
                }),
                ..EnvVarSource::default()
            }),
            ..EnvVar::default()
        };

        self.add_env_var(var)
    }

    pub fn build(mut self, task: &Task) -> Result<WorkflowTemplate> {
        for var in task.spec.cfg.env.clone() {
            self.add_env_var(EnvVar {
                name: var.key,
                value: Some(var.val),
                value_from: None,
            });
        }

        for secret in task.spec.cfg.secrets.clone() {
            if let Some(variant) = secret.variant {
                self.add_secret_env_var(variant);
            }
        }

        let script_template = ArgoScriptTemplate {
            source: self.script,
            container: serde_json::from_value(json!(
                    {
                      "image": self.ctx.executor_image,
                      "imagePullPolicy": self.ctx.task_image_pull_policy,
                      "command": ["bash"],
                      "volumeMounts": [{
                          "name": self.ctx.task_volume,
                          "mountPath": "/project",
                      }],
                      "env": self.env,
                      "resources": {
                         "limits":  task.spec.cfg.resources,
                    }
                    }
            ))?,
        };
        Ok(WorkflowTemplate {
            security_context: Some(serde_json::from_value(json!({
                "runAsUser": 1001,
                "fsGroup": 2000
            }
            ))?),
            script: Some(script_template),
            ..WorkflowTemplate::new(self.name)
        })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn can_add_pull_secret() {
        let mut wf = Workflow::default();

        assert_eq!(wf.spec.image_pull_secrets, None);

        let secret_name = "mysecret".to_string();
        wf.add_pull_secret(secret_name.clone());

        match wf.spec.image_pull_secrets {
            Some(ref secrets) => {
                assert_eq!(secrets.len(), 1);
                assert_eq!(secrets[0].name.as_ref().unwrap(), &secret_name);
            }
            None => panic!("failed to add pull secret"),
        }

        let second_secret_name = "mysecret2".to_string();
        wf.add_pull_secret(second_secret_name.clone());
        match wf.spec.image_pull_secrets {
            Some(secrets) => {
                assert_eq!(secrets.len(), 2);
                assert_eq!(secrets[1].name.as_ref().unwrap(), &second_secret_name);
            }
            None => panic!("failed to add pull secret"),
        }
    }
}
