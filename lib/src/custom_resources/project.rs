use std::{
    collections::{BTreeMap, HashMap},
    default::Default,
};

use crate::{
    ctrl::AmeResource,
    custom_resources::{data_set::DataSet, Error, Result},
    error::AmeError,
    grpc::{resource_map_conv, DataSetCfg, Model, ProjectCfg, ProjectStatus, TaskCfg, TaskRef},
};

use super::new_task::{ProjectSource, Task, TaskBuilder};

use k8s_openapi::{
    api::{
        apps::v1::{Deployment, DeploymentSpec},
        core::v1::{
            Container, ContainerPort, EnvVar, HTTPGetAction, PodSpec, PodTemplateSpec, Probe,
            ResourceRequirements, Service, ServicePort, ServiceSpec,
        },
        networking::v1::{
            HTTPIngressPath, HTTPIngressRuleValue, Ingress, IngressBackend, IngressRule,
            IngressServiceBackend, IngressSpec, IngressTLS, ServiceBackendPort,
        },
    },
    apimachinery::pkg::{
        apis::meta::v1::{LabelSelector, OwnerReference},
        util::intstr::IntOrString,
    },
};
use kube::{core::ObjectMeta, CustomResource, Resource, ResourceExt};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::debug;

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
    #[serde(flatten)]
    pub cfg: ProjectCfg,
    pub deletion_approved: bool,

    #[serde(default)]
    pub enable_triggers: Option<bool>,
}

pub fn local_name(name: String) -> String {
    if name.contains('.') {
        name.split('.')
            .map(String::from)
            .collect::<Vec<String>>()
            .get(1)
            .unwrap_or(&name.to_string())
            .to_owned()
    } else {
        name
    }
}

pub fn project_name(name: String) -> Option<String> {
    if !name.contains('.') {
        return None;
    }

    let splits: Vec<String> = name.split('.').map(String::from).collect();

    if splits.len() > 1 {
        return Some(splits[0].clone());
    }

    None
}

impl From<ProjectCfg> for ProjectSpec {
    fn from(cfg: ProjectCfg) -> Self {
        Self {
            cfg,
            deletion_approved: false,
            enable_triggers: Some(false),
        }
    }
}

pub fn generate_task_name(project_name: String, task_name: String) -> String {
    format!("{project_name}{task_name}")
}

pub fn generate_data_set_task_name(project_name: String, data_set_name: String) -> String {
    format!("{}{}_task", project_name, data_set_name)
}

impl Project {
    pub fn from_cfg(cfg: ProjectCfg) -> Self {
        Self {
            metadata: ObjectMeta {
                generate_name: Some(cfg.name.clone()),
                ..ObjectMeta::default()
            },

            spec: ProjectSpec::from(cfg),
            status: None,
        }
    }

    pub fn deletion_approved(&self) -> bool {
        self.spec.deletion_approved
    }

    pub fn approve_deletion(&mut self) {
        self.spec.deletion_approved = true;
    }

    pub fn get_data_set(&self, data_set_name: String) -> Option<DataSetCfg> {
        let data_set_name = local_name(data_set_name);

        self.clone()
            .spec
            .cfg
            .data_sets
            .into_iter()
            .find(|ds| ds.name == data_set_name)
    }

    // TODO: documentat that this assumes a local data set.
    pub fn generate_data_set(&self, data_set_name: String) -> crate::Result<DataSet> {
        let Some(cfg) = self.get_data_set(data_set_name.clone()) else {
            debug!("failed to get data set {}", data_set_name);
            return Err(AmeError::MissingDataSet(data_set_name, self.name_any()));
        };

        let mut data_set = DataSet::from_cfg(
            &format!("dataset{}{}", self.spec.cfg.name, data_set_name),
            cfg,
        );

        if let Some(repo) = self.annotations().get("gitrepository") {
            data_set
                .annotations_mut()
                .insert("gitrepository".to_string(), repo.to_string());
        }

        data_set.spec.project = Some(self.spec.cfg.name.clone());

        let Some(project_oref) = self.gen_owner_ref() else {
            return Err(AmeError::FailedToCreateOref(self.name_any()));
        };

        data_set.owner_references_mut().push(project_oref);

        Ok(data_set)
    }

    pub fn generate_data_set_task(&self, data_set_name: String) -> Option<Task> {
        let Some(Some(task)) = self.get_data_set(data_set_name.clone()).map(|ds| ds.task) else {
            return None;
        };

        let mut task_builder = TaskBuilder::from_cfg(task);

        task_builder.set_name(generate_data_set_task_name(
            self.spec.cfg.name.clone(),
            data_set_name,
        ));
        task_builder.set_project(self.spec.cfg.name.clone());

        if let Some(repo) = self.annotations().get("gitrepository") {
            task_builder.set_project_src(ProjectSource::from_public_git_repo(repo.to_string()));
        }

        Some(task_builder.build())
    }

    pub fn generate_validation_task(&self, model: &Model, latest_version: String) -> Result<Task> {
        let cfg = if let Some(cfg) = model.validation_task.as_ref() {
            if let Some(ref task_ref) = cfg.task_ref {
                self.find_task_cfg(&task_ref.name)
                    .ok_or(Error::MissingTaskCfg(
                        task_ref.name.clone(),
                        self.spec.cfg.name.clone(),
                    ))?
            } else {
                cfg.to_owned()
            }
        } else {
            return Err(Error::MissingValidationTask(model.name.clone()));
        };

        let mut task_builder = TaskBuilder::from_cfg(cfg);

        if let Some(repo) = self.annotations().get("gitrepository") {
            task_builder.set_project_src(ProjectSource::from_public_git_repo(repo.to_string()));
        }

        task_builder.add_owner_reference(
            self.controller_owner_ref(&())
                .ok_or(AmeError::FailedToCreateOref(self.name_any()))?,
        );

        let validation_task = task_builder
            .set_name(format!(
                "validate-{}-{}",
                model.name.clone(),
                latest_version
            ))
            .clone()
            .build();

        if validation_task.spec.cfg.executor.is_none() {
            return Err(Error::MissingExecutor(validation_task.name_any()));
        }

        Ok(validation_task)
    }

    pub fn add_owner_reference(&mut self, owner_reference: OwnerReference) -> &mut Project {
        match &mut self.metadata.owner_references {
            Some(refs) => refs.push(owner_reference),
            None => self.metadata.owner_references = Some(vec![owner_reference]),
        };

        self
    }

    pub fn add_annotation(&mut self, key: String, val: String) -> &mut Project {
        let mut annotations = if let Some(annotations) = self.metadata.annotations.clone() {
            annotations
        } else {
            BTreeMap::new()
        };

        annotations.insert(key, val);

        self.metadata.annotations = Some(annotations);
        self
    }

    fn get_model(&self, name: &str) -> Option<Model> {
        self.spec
            .cfg
            .models
            .clone()
            .into_iter()
            .find(|m| m.name == name)
    }

    pub fn get_template(&self, name: &str) -> Option<TaskCfg> {
        self.spec.cfg.templates.clone().into_iter().find(|m| {
            if let Some(ref tname) = m.name {
                tname == name
            } else {
                false
            }
        })
    }

    fn find_task_cfg(&self, name: &str) -> Option<TaskCfg> {
        self.spec
            .cfg
            .tasks
            .iter()
            .find(|task| task.name.as_ref().map(|n| n == name).unwrap_or(false))
            .cloned()
    }

    fn get_task_cfg_for_ref(&self, task_ref: TaskRef) -> Option<TaskCfg> {
        self.spec.cfg.get_task_cfg(&task_ref.name)
    }

    fn get_model_training_cfg(&self, model: &str) -> Result<TaskCfg> {
        let Some(model) = self.get_model(model) else {
            return Err(
                AmeError::MissingModelTrainingTaskCfg(model.to_string(), self.name_any()).into(),
            );
        };

        match model.get_training_task_cfg() {
            Some(TaskCfg {
                task_ref: Some(ref task_ref),
                ..
            }) => self
                .get_task_cfg_for_ref(task_ref.clone())
                .ok_or(AmeError::MissingTaskRef(task_ref.name.to_string()).into()),
            Some(task_cfg @ TaskCfg { task_ref: None, .. }) => Ok(task_cfg),
            None => Err(AmeError::MissingTrainingTaskCfg(model.name).into()),
        }
    }

    pub fn generate_model_training_task(&self, name: &str) -> Result<Task> {
        let task_cfg = self.get_model_training_cfg(name)?;

        let mut task_builder = TaskBuilder::from_cfg(task_cfg.clone());

        let medata = self.metadata.clone();

        let annotations = medata.annotations.unwrap();

        let repo = annotations.get("gitrepository").unwrap();
        // NOTE: how is the data set repo set as src here?

        let training_task = task_builder
            .set_project_src(ProjectSource::from_public_git_repo(repo.to_string()))
            .set_name(format!(
                "{}{}{}{}",
                self.name_any(),
                name,
                task_cfg.name(),
                self.spec.cfg.name
            ))
            .add_owner_reference(
                self.controller_owner_ref(&())
                    .unwrap_or(OwnerReference::default()),
            )
            .clone()
            .build();

        Ok(training_task)
    }
}

pub fn add_owner_reference(
    mut metadata: ObjectMeta,
    owner_reference: OwnerReference,
) -> ObjectMeta {
    match &mut metadata.owner_references {
        Some(refs) => refs.push(owner_reference),
        None => metadata.owner_references = Some(vec![owner_reference]),
    };

    metadata
}

pub async fn get_latest_model_version(
    model: &Model,
    mlflow_url: String,
) -> Result<MlflowModelVersion> {
    let Some(model_version) = ({
        let mut body = HashMap::new();
        body.insert("name", model.name.clone());
        let client = reqwest::Client::new();
        let MlflowModelVersionsRes { model_versions } = client
            .post(format!(
                "{mlflow_url}/api/2.0/mlflow/registered-models/get-latest-versions"
            ))
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        model_versions
            .into_iter()
            .max_by_key(|v| v.creation_timestamp)
    }) else {
        return Err(Error::MissingMlflowUrl());
    };

    Ok(model_version)
}

impl Model {
    fn get_training_task_cfg(&self) -> Option<TaskCfg> {
        self.training.as_ref().and_then(|t| t.task.clone())
    }

    fn labels(&self) -> BTreeMap<String, String> {
        let mut labels: BTreeMap<String, String> = BTreeMap::new();
        labels.insert("ame-model".to_string(), self.name.clone());
        labels
    }

    fn object_metadata(&self) -> ObjectMeta {
        ObjectMeta {
            name: Some(self.name.clone()),
            labels: Some(self.labels().to_owned()),
            ..ObjectMeta::default()
        }
    }

    pub fn generate_model_ingress(
        &self,
        ingress_host: String,
        ingress_annotations: Option<BTreeMap<String, String>>,
        project_name: String,
    ) -> Result<Ingress> {
        let Some(mut model_deployment) = self.deployment.clone() else {
            return Err(Error::MissingDeployment());
        };

        let mut ingress_annotations = ingress_annotations
            .clone()
            .unwrap_or(BTreeMap::<String, String>::new());

        ingress_annotations.insert(
            "nginx.ingress.kubernetes.io/ssl-redirect".to_string(),
            "false".to_string(),
        );

        ingress_annotations.append(&mut model_deployment.ingress_annotations);

        // TODO: we need a better method of setting paths for models as this is can easily break.
        ingress_annotations.insert(
            "nginx.ingress.kubernetes.io/rewrite-target".to_string(),
            "/$2".to_string(),
        );

        let metadata = ObjectMeta {
            name: Some(self.name.clone()),
            labels: Some(self.labels()),
            annotations: Some(ingress_annotations),
            ..ObjectMeta::default()
        };

        let tls: Option<Vec<IngressTLS>> = match model_deployment.enable_tls {
            Some(true) | None => Some(vec![IngressTLS {
                hosts: Some(vec![ingress_host.clone()]),
                secret_name: Some(format!("{}-tls", self.name)),
            }]),
            _ => None,
        };

        Ok(Ingress {
            metadata,
            spec: Some(IngressSpec {
                ingress_class_name: Some("nginx".to_string()),
                rules: Some(vec![IngressRule {
                    host: Some(ingress_host),
                    http: Some(HTTPIngressRuleValue {
                        paths: vec![HTTPIngressPath {
                            backend: IngressBackend {
                                service: Some(IngressServiceBackend {
                                    name: self.name.clone(),
                                    port: Some(ServiceBackendPort {
                                        number: Some(5000),
                                        name: None,
                                    }),
                                }),
                                ..IngressBackend::default()
                            },
                            path_type: "Prefix".to_string(),
                            path: Some(format!(
                                "/projects/{}/models/{}(/|$)(.*)",
                                project_name, self.name
                            )),
                        }],
                    }),
                }]),
                tls,
                ..IngressSpec::default()
            }),
            ..Ingress::default()
        })
    }

    pub fn generate_model_service(&self) -> Result<Service> {
        let Some(_model_deployment) = self.deployment.clone() else {
            return Err(Error::MissingDeployment());
        };

        Ok(Service {
            metadata: self.object_metadata(),
            spec: Some(ServiceSpec {
                selector: Some(self.labels()),
                ports: Some(vec![ServicePort {
                    port: 5000,
                    ..ServicePort::default()
                }]),
                ..ServiceSpec::default()
            }),
            ..Service::default()
        })
    }

    pub async fn get_model_version(
        &self,
        _ctrl_cfg: &ProjectCtrlCfg,
        _version: &str,
    ) -> Result<MlflowModelVersion> {
        todo!();
    }

    pub async fn generate_model_deployment(
        &self,
        deployment_image: String,
        model_source: String,
    ) -> Result<Deployment> {
        let Some(model_deployment) = self.deployment.clone() else {
            return Err(Error::MissingDeployment());
        };

        let labels = self.labels();

        let server_port = 5000;

        Ok(Deployment {
            metadata: ObjectMeta {
                name: Some(self.name.clone()),
                labels: Some(labels.clone()),
                ..ObjectMeta::default()
            },
            spec: Some(DeploymentSpec {
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    ..LabelSelector::default()
                },
                replicas: Some(model_deployment.replicas.unwrap_or(1)),
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(labels.clone()),
                        ..ObjectMeta::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            security_context: Some(serde_json::from_value(json!({
                            "runAsUser": 1001,
                            "fsGroup": 2000
                            }))?),
                            name: "main".to_string(),
                            image: Some(
                                model_deployment
                                    .image
                                    .unwrap_or(deployment_image),
                            ),
                            command: Some(vec!["/bin/bash".to_string()]),
                            args: Some(vec![
                                "-c".to_string(),
                                format!("export PATH=$HOME/.pyenv/bin:$PATH; mlflow models serve -m {model_source} --host 0.0.0.0"),
                            ]),
                            resources: Some(ResourceRequirements{
                                limits: Some(resource_map_conv(model_deployment.resources)),
                                requests: None,
                            }),
                            env: Some(vec![EnvVar {
                                name: "MLFLOW_TRACKING_URI".to_string(),
                                value: Some(
                                    "http://mlflow.ame-system.svc.cluster.local:5000".to_string(),
                                ),
                                ..EnvVar::default()
                            }]),
                            ports: Some(vec![ContainerPort {
                                container_port: server_port,
                                ..ContainerPort::default()
                            }]),
                            readiness_probe: Some(Probe {
                                http_get: Some(HTTPGetAction {
                                    port: IntOrString::Int(server_port),
                                    path: Some("/health".to_string()),
                                    ..HTTPGetAction::default()
                                }),
                                ..Probe::default()
                            }),
                            ..Container::default()
                        }],
                        ..PodSpec::default()
                    }),
                },
                ..DeploymentSpec::default()
            }),
            ..Deployment::default()
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
pub struct ProjectCtrlCfg {
    namespace: String,
    deployment_image: String,
    model_deployment_ingress: Option<Ingress>,
    model_ingress_annotations: Option<BTreeMap<String, String>>,
    model_ingress_host: Option<String>,
    mlflow_url: Option<String>,
}

impl ProjectCtrlCfg {
    pub fn from_env() -> Result<Self> {
        let prefix = "AME";

        Ok(ProjectCtrlCfg {
            namespace: std::env::var(format!("{prefix}_NAMESPACE"))
                .unwrap_or("ame-system".to_string()),
            deployment_image: std::env::var("EXECUTOR_IMAGE")
                .unwrap_or("main.localhost:45373/ame-executor:latest".to_string()),
            model_deployment_ingress: serde_yaml::from_str(
                &std::env::var(format!("{prefix}_MODEL_DEPLOYMENT_INGRESS"))
                    .unwrap_or("".to_string()),
            )
            .ok(),
            model_ingress_annotations: Some(BTreeMap::new()),
            model_ingress_host: std::env::var(format!("{prefix}_MODEL_INGRESS_HOST")).ok(),
            mlflow_url: std::env::var(format!("{prefix}_MLFLOW_URL")).ok(),
        })
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
struct MlflowModelVersionsRes {
    model_versions: Vec<MlflowModelVersion>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
struct MlflowModelVersionResponse {
    model_version: MlflowModelVersion,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Default)]
pub struct MlflowModelVersion {
    name: String,
    pub version: String,
    current_stage: String,
    creation_timestamp: i64,
    pub source: String,
    run_id: String,
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use serde_json::json;

    use super::{Project, ProjectCtrlCfg, Result};
    use serial_test::serial;

    fn test_project() -> Result<Project> {
        Ok(serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Project",
            "metadata": { "name": "test-private", "annotations": {
                "gitrepository": "my-git-repo"
            } },
            "spec": {
                "deletionApproved": false,

                "name": "myproject",
                "deletionApproved": false,
                "models": [
                {
                    "name": "test",
                    "training": {
                        "task": {
                            "name": "mytask",
                            "executor": {
                                 "pipEnv": {
                                        "command": "test cmd"
                                    }
                                },
                        }
                    },
                    "deployment": {
                        "deploy": true,
                        "auto_train": false
                    }
                }
                ]
            }
        }))?)
    }

    #[tokio::test]
    #[serial]
    async fn produces_valid_deployment() -> Result<()> {
        let ctrl_cfg = ProjectCtrlCfg {
            namespace: "default".to_string(),
            deployment_image: "test_img".to_string(),
            model_ingress_host: Some("testhost".to_string()),
            mlflow_url: Some("mlflowurl".to_string()),
            ..ProjectCtrlCfg::default()
        };
        tokio::time::sleep(Duration::from_secs(2)).await;

        let project = test_project()?;

        insta::assert_yaml_snapshot!(
            &project.spec.cfg.models.clone()[0]
                .generate_model_deployment(ctrl_cfg.deployment_image, "model_source".to_string())
                .await?
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn produces_valid_model_training_task() -> Result<()> {
        let project = test_project().unwrap();
        insta::assert_yaml_snapshot!(&project.generate_model_training_task("test")?);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn produces_valid_service() -> Result<()> {
        let _ctrl_cfg = ProjectCtrlCfg {
            namespace: "default".to_string(),
            deployment_image: "test_img".to_string(),
            model_ingress_host: Some("testhost".to_string()),
            ..ProjectCtrlCfg::default()
        };
        tokio::time::sleep(Duration::from_secs(2)).await;

        let project = test_project()?;

        insta::assert_yaml_snapshot!(&project.spec.cfg.models[0].generate_model_service()?);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn produces_valid_ingress() -> Result<()> {
        let _ctrl_cfg = ProjectCtrlCfg {
            namespace: "default".to_string(),
            deployment_image: "test_img".to_string(),
            model_ingress_host: Some("testhost".to_string()),
            ..ProjectCtrlCfg::default()
        };
        tokio::time::sleep(Duration::from_secs(2)).await;

        let project = test_project()?;

        insta::assert_yaml_snapshot!(&project.spec.cfg.models[0].generate_model_ingress(
            "".to_string(),
            None,
            "projectname".to_string()
        )?);

        Ok(())
    }
}
