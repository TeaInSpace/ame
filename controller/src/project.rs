use std::{
    collections::{BTreeMap, HashMap},
    default::Default,
    sync::Arc,
    time::Duration,
};

use crate::{
    manager::{self, TaskPhase},
    Error, Result, TaskSpec,
};

use ame::grpc::LogEntry;
use futures::{future::BoxFuture, FutureExt, StreamExt};
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
        apis::meta::v1::{LabelSelector, OwnerReference, Time},
        util::intstr::IntOrString,
    },
};
use kube::{
    api::{ListParams, Patch, PatchParams, PostParams},
    core::ObjectMeta,
    runtime::{controller::Action, Controller},
    Api, Client, CustomResource, Resource, ResourceExt,
};
use reqwest::Url;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, error, log::info};

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
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<Model>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<TaskSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<Vec<TaskSpec>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_entry: Option<LogEntry>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Default)]
pub struct ProjectStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<BTreeMap<String, ModelStatus>>,
}

impl ProjectStatus {
    fn set_model_status(&mut self, name: &str, status: ModelStatus) {
        if let Some(ref mut statuses) = self.models {
            statuses.insert(name.to_string(), status);
        } else {
            let mut statuses: BTreeMap<String, ModelStatus> = BTreeMap::new();
            statuses.insert(name.to_string(), status);
            self.models = Some(statuses);
        }
    }

    fn set_model_validation(&mut self, name: &str, validation: ModelValidationStatus) {
        let mut default = ModelStatus::default();
        let mut status = self.get_model_status(name).unwrap_or(&mut default).clone();

        status.validation = Some(validation);

        self.set_model_status(name, status);
    }

    pub fn get_model_status(&mut self, name: &str) -> Option<&mut ModelStatus> {
        self.models.as_mut().and_then(|models| models.get_mut(name))
    }

    fn set_latest_valid_model_version(&mut self, name: &str, version: String) {
        let mut status = self
            .get_model_status(name)
            .map(|s| s.to_owned())
            .unwrap_or_default();

        status.latest_valid_model_version = Some(version);
        self.set_model_status(name, status)
    }

    fn get_latest_valid_model_version(&mut self, name: &str) -> Option<String> {
        self.get_model_status(name)
            .as_ref()
            .and_then(|model_status| model_status.latest_valid_model_version.clone())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_type: Option<ModelType>,

    training: TrainingCfg,
    #[serde(skip_serializing_if = "Option::is_none")]
    deployment: Option<ModelDeployment>,

    #[serde(skip_serializing_if = "Option::is_none")]
    validation_task: Option<TaskSpec>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
pub struct TrainingCfg {
    task: TaskSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
pub struct ModelDeployment {
    deploy: bool,
    auto_train: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resources: Option<ResourceRequirements>,
    #[serde(skip_serializing_if = "Option::is_none")]
    replicas: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ingress: Option<Ingress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ingress_annotations: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_tls: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum ModelType {
    Mlflow,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_model_version: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_validated_model_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_trained: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_deployed: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ModelValidationStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_valid_model_version: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModelValidationStatus {
    UnValidated {}, // we cannot use a unit type here: see https://docs.rs/kube/latest/kube/derive.CustomResource.html#enums
    Validating { task: String, model_version: String },
    FailedValidation { model_version: String },
    Validated { model_version: String },
}

impl Default for ModelValidationStatus {
    fn default() -> Self {
        ModelValidationStatus::UnValidated {}
    }
}

impl Project {
    fn gen_validation_task(&self, model: &Model) -> Result<manager::Task> {
        let Some(mut spec) =match model.validation_task.as_ref() {
            Some(TaskSpec {
                task_ref: Some(name),
                ..
            }) => self.find_task_spec(name),
            Some(task_spec) => Some(task_spec),
            None => None
        }.map(|ts| ts.to_owned()) else {
            return Err(Error::MissingValidationTask(model.name.clone()));
        };

        if let Some(repo) = self.annotations().get("gitrepository") {
            spec.source = Some(manager::ProjectSource {
                gitrepository: Some(repo.to_owned()),
                ..manager::ProjectSource::default()
            });
        }

        let metadata = ObjectMeta {
            generate_name: Some(format!("validate-{}", model.name.clone())),
            ..ObjectMeta::default()
        };

        Ok(manager::Task {
            metadata,
            spec,
            status: None,
        })
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
        if let Some(ref models) = self.spec.models {
            models.clone().into_iter().find(|m| m.name == name)
        } else {
            None
        }
    }

    pub fn get_template(&self, name: &str) -> Option<TaskSpec> {
        if let Some(ref templates) = self.spec.templates {
            templates.clone().into_iter().find(|m| {
                if let Some(ref tname) = m.name {
                    tname == name
                } else {
                    false
                }
            })
        } else {
            None
        }
    }

    fn find_task_spec(&self, name: &str) -> Option<&manager::TaskSpec> {
        self.spec.tasks.as_ref().and_then(|tasks| {
            tasks
                .iter()
                .find(|task| task.name.as_ref().map(|n| n == name).unwrap_or(false))
        })
    }

    pub fn generate_model_training_task(&self, name: &str) -> Result<manager::Task> {
        let Some(model) = self.get_model(name) else {
            return Err(Error::MissingProjectSrc("model".to_string()));
        };

        let task_ref = model.training.task.task_ref.clone().unwrap();

        let project_spec = self.spec.clone();
        let tasks = project_spec.tasks.unwrap();
        let mut task_spec = tasks
            .iter()
            .find(|t| t.name.clone().unwrap() == task_ref)
            .unwrap()
            .to_owned();

        let medata = self.metadata.clone();

        let annotations = medata.annotations.unwrap();

        let repo = annotations.get("gitrepository").unwrap();

        task_spec.source = Some(manager::ProjectSource {
            gitrepository: Some(repo.to_owned()),
            ..manager::ProjectSource::default()
        });

        let metadata = ObjectMeta {
            generate_name: Some(model.name),
            ..ObjectMeta::default()
        };

        Ok(manager::Task {
            metadata: add_owner_reference(metadata, self.controller_owner_ref(&()).unwrap()),
            spec: task_spec,
            status: None,
        })
    }
}

fn add_owner_reference(mut metadata: ObjectMeta, owner_reference: OwnerReference) -> ObjectMeta {
    match &mut metadata.owner_references {
        Some(refs) => refs.push(owner_reference),
        None => metadata.owner_references = Some(vec![owner_reference]),
    };

    metadata
}

impl Model {
    fn labels(&self) -> BTreeMap<String, String> {
        let mut labels: BTreeMap<String, String> = BTreeMap::new();
        labels.insert("ame-model".to_string(), self.name.clone());
        labels
    }

    fn object_metadata(&self) -> ObjectMeta {
        ObjectMeta {
            name: Some(self.name.clone()),
            labels: Some(self.labels()),
            ..ObjectMeta::default()
        }
    }

    fn generate_model_ingress(&self, ctrl_cfg: &ProjectCtrlCfg) -> Result<Ingress> {
        let Some(model_deployment) = self.deployment.clone() else {
            return Err(Error::MissingDeployment())
        };

        let mut ingress_annotations = ctrl_cfg
            .model_ingress_annotations
            .clone()
            .unwrap_or(BTreeMap::<String, String>::new());

        ingress_annotations.insert(
            "nginx.ingress.kubernetes.io/ssl-redirect".to_string(),
            "false".to_string(),
        );

        if let Some(mut annotations) = model_deployment.ingress_annotations {
            ingress_annotations.append(&mut annotations);
        }

        let metadata = ObjectMeta {
            name: Some(self.name.clone()),
            labels: Some(self.labels()),
            annotations: Some(ingress_annotations),
            ..ObjectMeta::default()
        };

        let tls: Option<Vec<IngressTLS>> = match model_deployment.enable_tls {
            Some(true) | None => Some(vec![IngressTLS {
                hosts: Some(vec![ctrl_cfg
                    .clone()
                    .model_ingress_host
                    .unwrap_or("".to_string())]),
                secret_name: Some(format!("{}-tls", self.name)),
            }]),
            _ => None,
        };

        Ok(Ingress {
            metadata,
            spec: Some(IngressSpec {
                ingress_class_name: Some("nginx".to_string()),
                rules: Some(vec![IngressRule {
                    host: Some(
                        ctrl_cfg
                            .clone()
                            .model_ingress_host
                            .unwrap_or("".to_string()),
                    ),
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
                            path_type: "ImplementationSpecific".to_string(),
                            path: Some("/invocations".to_string()),
                        }],
                    }),
                }]),
                tls,
                ..IngressSpec::default()
            }),
            ..Ingress::default()
        })
    }

    fn generate_model_service(&self, _ctrl_cfg: &ProjectCtrlCfg) -> Result<Service> {
        let Some(_model_deployment) = self.deployment.clone() else {
            return Err(Error::MissingDeployment())
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

    async fn get_model_version(
        &self,
        ctrl_cfg: &ProjectCtrlCfg,
        version: &str,
    ) -> Result<MlflowModelVersion> {
        let Some(ref mlflow_url) = ctrl_cfg.mlflow_url else {
            return Err(Error::MissingMlflowUrl());
        };

        let model_version = {
            let mut body = HashMap::new();
            body.insert("name", self.name.clone());
            body.insert("version", version.to_string());
            let url = Url::parse_with_params(
                &format!("{mlflow_url}/api/2.0/mlflow/model-versions/get"),
                &[
                    ("name", self.name.clone()),
                    ("version", version.to_string()),
                ],
            )
            .unwrap();

            let client = reqwest::Client::new();
            client
                .get(url)
                .send()
                .await?
                .json::<MlflowModelVersionResponse>()
                .await?
                .model_version
        };
        Ok(model_version)
    }

    async fn get_latest_model_version(
        &self,
        ctrl_cfg: &ProjectCtrlCfg,
    ) -> Result<MlflowModelVersion> {
        let Some(ref mlflow_url) = ctrl_cfg.mlflow_url else {
            return Err(Error::MissingMlflowUrl());
        };

        let Some(model_version) = ({
            let mut body = HashMap::new();
            body.insert("name", self.name.clone());
            let client = reqwest::Client::new();
            let MlflowModelVersionsRes{model_versions } = client
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

    async fn generate_model_deployment(
        &self,
        ctrl_cfg: &ProjectCtrlCfg,
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
                        labels: Some(labels),
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
                                    .unwrap_or(ctrl_cfg.clone().deployment_image),
                            ),
                            command: Some(vec!["/bin/bash".to_string()]),
                            args: Some(vec![
                                "-c".to_string(),
                                format!("export PATH=$HOME/.pyenv/bin:$PATH; mlflow models serve -m {model_source} --host 0.0.0.0"),
                            ]),
                            resources: model_deployment.resources,
                            env: Some(vec![EnvVar {
                                name: "MLFLOW_TRACKING_URI".to_string(),
                                value: Some(
                                    "http://mlflow.default.svc.cluster.local:5000".to_string(),
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
            namespace: std::env::var(format!("{prefix}_NAMESPACE"))?,
            deployment_image: std::env::var("EXECUTOR_IMAGE")?,
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

struct Context {
    client: Client,
    config: ProjectCtrlCfg,
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
struct MlflowModelVersion {
    name: String,
    version: String,
    current_stage: String,
    creation_timestamp: i64,
    source: String,
    run_id: String,
}

async fn reconcile(project: Arc<Project>, ctx: Arc<Context>) -> Result<Action> {
    info!("reconciling projects");
    let _projects = Api::<Project>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let deployments = Api::<Deployment>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let ingresses = Api::<Ingress>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let services = Api::<Service>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let tasks_cli = Api::<manager::Task>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let ctrl_cfg = ctx.config.clone();
    let oref = if let Some(refe) = project.controller_owner_ref(&()) {
        refe
    } else {
        OwnerReference::default()
    };

    let mut status = project.status.clone().unwrap_or_default();

    if let Some(models) = project.spec.clone().models {
        for model in models {
            let model_version = model.get_latest_model_version(&ctrl_cfg).await;

            if let Some(ModelDeployment {
                auto_train: true,
                deploy: true,
                ..
            }) = model.clone().deployment
            {
                info!("creating autotrain task {model_version:?}");
                if model_version.is_err() {
                    let task_ref = model.training.task.task_ref.clone().unwrap();

                    let project_spec = project.spec.clone();
                    let tasks = project_spec.tasks.unwrap();
                    let mut task_spec = tasks
                        .iter()
                        .find(|t| t.name.clone().unwrap() == task_ref)
                        .unwrap()
                        .to_owned();

                    let medata = project.metadata.clone();

                    let annotations = medata.annotations.unwrap();

                    let repo = annotations.get("gitrepository").unwrap();

                    task_spec.source = Some(manager::ProjectSource {
                        gitrepository: Some(repo.to_owned()),
                        ..manager::ProjectSource::default()
                    });

                    let metadata = ObjectMeta {
                        name: Some(model.name.clone()),
                        ..ObjectMeta::default()
                    };

                    let task = manager::Task {
                        metadata: add_owner_reference(metadata, oref.clone()),
                        spec: task_spec.clone(),
                        status: None,
                    };

                    tasks_cli
                        .patch(
                            &task.name_any(),
                            &PatchParams::apply("ame"),
                            &Patch::Apply(&task),
                        )
                        .await?;
                }
            };

            debug!(
                "check model validation status {:?} {:?}",
                model_version, model.validation_task
            );

            if model.validation_task.is_some() && model_version.is_ok() {
                let validation = status
                    .clone()
                    .get_model_status(&model.name)
                    .and_then(|s| s.validation.as_ref())
                    .unwrap_or(&ModelValidationStatus::UnValidated {})
                    .clone();

                debug!("update model validation state");

                match validation {
                    ModelValidationStatus::UnValidated {} => {
                        debug!(
                            "model {} with version {} needs to be validated",
                            &model.name,
                            model_version
                                .as_ref()
                                .map(|m| m.version.clone())
                                .unwrap_or("'no version found'".to_string())
                        );

                        let mut validation_task = project.gen_validation_task(&model)?; // TODO: the ? here could cause unwanted early exits.
                        validation_task.owner_references_mut().push(oref.clone());

                        let validation_task = tasks_cli
                            .create(&PostParams::default(), &validation_task)
                            .await?;

                        status.set_model_validation(
                            &model.name,
                            ModelValidationStatus::Validating {
                                task: validation_task.name_any(),
                                model_version: model_version.as_ref().unwrap().version.clone(),
                            },
                        );
                    }
                    ModelValidationStatus::Validating {
                        task,
                        model_version,
                    } => {
                        // TODO: handle early cancelation if a new model version appears before
                        // this validation completes.

                        debug!("debug status: validating");

                        let Ok(task_status) = tasks_cli.get_status(&task).await else {
                            debug!("failed to find status for task {task}, resetting model status to unvalidatd");
                            status.set_model_validation(&model.name, ModelValidationStatus::UnValidated {  });
                            continue;
                        };

                        debug!("validation task status: {task_status:?}");

                        if let Some(task_status) = task_status.status {
                            match task_status.phase {
                                Some(TaskPhase::Succeeded) => {
                                    status.set_latest_valid_model_version(
                                        &model.name,
                                        model_version.clone(),
                                    );
                                    status.set_model_validation(
                                        &model.name,
                                        ModelValidationStatus::Validated {
                                            model_version: model_version.to_string(),
                                        },
                                    );
                                }
                                Some(TaskPhase::Failed) => status.set_model_validation(
                                    &model.name,
                                    ModelValidationStatus::FailedValidation {
                                        model_version: model_version.to_string(),
                                    },
                                ),
                                _ => (),
                            };

                            debug!("set model validations status: {status:?}");
                        }
                    }

                    ModelValidationStatus::Validated {
                        model_version: validatd_version,
                    } => {
                        if *validatd_version == model_version.as_ref().unwrap().version {
                            debug!("new model version requires validation");
                            let mut validation_task = project.gen_validation_task(&model)?; // TODO: the ? here could cause unwanted early exits.
                            validation_task.owner_references_mut().push(oref.clone());

                            let validation_task = tasks_cli
                                .create(&PostParams::default(), &validation_task)
                                .await?;

                            status.set_model_validation(
                                &model.name,
                                ModelValidationStatus::Validating {
                                    task: validation_task.name_any(),
                                    model_version: model_version.as_ref().unwrap().version.clone(),
                                },
                            );

                            continue;
                        }
                    }
                    ModelValidationStatus::FailedValidation {
                        model_version: validatd_version,
                    } => {
                        if *validatd_version != model_version.as_ref().unwrap().version {
                            debug!("failed to validate");
                            let mut validation_task = project.gen_validation_task(&model)?; // TODO: the ? here could cause unwanted early exits.
                            validation_task.owner_references_mut().push(oref.clone());

                            let validation_task = tasks_cli
                                .create(&PostParams::default(), &validation_task)
                                .await?;

                            status.set_model_validation(
                                &model.name,
                                ModelValidationStatus::Validating {
                                    task: validation_task.name_any(),
                                    model_version: model_version.as_ref().unwrap().version.clone(),
                                },
                            );
                        }
                        continue;
                    }
                }
            }

            debug!("preparing model deployment");

            let Some(latest_version) = status.get_latest_valid_model_version(&model.name) else {
                debug!("no valid model version to deploy");
                continue;
            };

            let mlflow_version = model.get_model_version(&ctrl_cfg, &latest_version).await?;

            let mut deployment = model
                .generate_model_deployment(&ctrl_cfg, mlflow_version.source)
                .await?;
            let mut service = model.generate_model_service(&ctrl_cfg)?;
            let mut ingress = model.generate_model_ingress(&ctrl_cfg)?;
            let pp = PatchParams::apply("ame");

            deployment.metadata = add_owner_reference(deployment.metadata, oref.clone());
            service.metadata = add_owner_reference(service.metadata, oref.clone());
            ingress.metadata = add_owner_reference(ingress.metadata, oref.clone());

            deployments
                .patch(&deployment.name_any(), &pp, &Patch::Apply(&deployment))
                .await?;
            services
                .patch(&service.name_any(), &pp, &Patch::Apply(&service))
                .await?;
            ingresses
                .patch(&ingress.name_any(), &pp, &Patch::Apply(&ingress))
                .await?;
        }
    }

    debug!("patching status");

    let mut patch = Project {
        metadata: project.metadata.clone(),
        spec: ProjectSpec::default(),
        status: Some(status),
    };

    patch.meta_mut().managed_fields = None;

    _projects
        .patch_status(
            &project.name_any(),
            &PatchParams::apply("ame").force(),
            &Patch::Apply(patch),
        )
        .await?;

    debug!("requeing");

    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(src: Arc<Project>, error: &Error, _ctx: Arc<Context>) -> Action {
    error!("error: {}, for project: {}", error, src.name_any());
    Action::requeue(Duration::from_secs(60))
}

pub async fn start_project_controller(config: ProjectCtrlCfg) -> BoxFuture<'static, ()> {
    let client = Client::try_default().await.expect("Failed to create a K8S client, is the controller running in an environment with access to cluster credentials?");
    let context = Arc::new(Context {
        client: client.clone(),
        config,
    });

    let projects = Api::<Project>::namespaced(client.clone(), &context.config.namespace);
    projects
        .list(&ListParams::default())
        .await
        .expect("Is the CRD installed?");

    let services = Api::<Service>::namespaced(client.clone(), &context.config.namespace);
    let ingresses = Api::<Ingress>::namespaced(client.clone(), &context.config.namespace);
    let deployments = Api::<Deployment>::namespaced(client.clone(), &context.config.namespace);
    let tasks = Api::<manager::Task>::namespaced(client, &context.config.namespace);

    Controller::new(projects.clone(), ListParams::default())
        .owns(deployments, ListParams::default())
        .owns(services, ListParams::default())
        .owns(ingresses, ListParams::default())
        .owns(tasks, ListParams::default())
        .run(reconcile, error_policy, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .boxed()
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use serde_json::json;

    use super::{Project, ProjectCtrlCfg};
    use crate::Result;
    use serial_test::serial;

    fn test_project() -> Result<Project> {
        Ok(serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Project",
            "metadata": { "name": "test-private" },
            "spec": {
                "projectid": "myproject",
                "models": [
                {
                    "name": "test",
                    "training": {
                        "task": {
                            "taskRef": "trainingtask",
                            "runcommand": "test",
                            "projectid": "test",
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
            &project.spec.models.unwrap().clone()[0]
                .generate_model_deployment(&ctrl_cfg, "model_source".to_string())
                .await?
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn produces_valid_service() -> Result<()> {
        let ctrl_cfg = ProjectCtrlCfg {
            namespace: "default".to_string(),
            deployment_image: "test_img".to_string(),
            model_ingress_host: Some("testhost".to_string()),
            ..ProjectCtrlCfg::default()
        };
        tokio::time::sleep(Duration::from_secs(2)).await;

        let project = test_project()?;

        insta::assert_yaml_snapshot!(
            &project.spec.models.unwrap()[0].generate_model_service(&ctrl_cfg)?
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn produces_valid_ingress() -> Result<()> {
        let ctrl_cfg = ProjectCtrlCfg {
            namespace: "default".to_string(),
            deployment_image: "test_img".to_string(),
            model_ingress_host: Some("testhost".to_string()),
            ..ProjectCtrlCfg::default()
        };
        tokio::time::sleep(Duration::from_secs(2)).await;

        let project = test_project()?;

        insta::assert_yaml_snapshot!(
            &project.spec.models.unwrap()[0].generate_model_ingress(&ctrl_cfg)?
        );

        Ok(())
    }
}
