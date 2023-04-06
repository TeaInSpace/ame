use crate::add_owner_reference;
use crate::argo::ArgoScriptTemplate;
use crate::argo::WorkflowStep;
use crate::argo::WorkflowTemplate;
use crate::local_name;
use crate::project;
use crate::DataSet;
use crate::Workflow;
use crate::WorkflowPhase;
use crate::{Error, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use futures::StreamExt;

use ame::grpc::TaskRef;

use futures::future::join_all;
use k8s_openapi::api::core::v1::EnvVar;
use k8s_openapi::api::core::v1::EnvVarSource;

use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use k8s_openapi::api::core::v1::PersistentVolumeClaimSpec;
use k8s_openapi::api::core::v1::PersistentVolumeClaimStatus;
use k8s_openapi::api::core::v1::ResourceRequirements;

use k8s_openapi::api::core::v1::SecretKeySelector;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::api::Patch;
use kube::api::PatchParams;
use kube::core::object::HasStatus;
use kube::core::ObjectMeta;
use kube::error::ErrorResponse;
use kube::{
    api::{Api, ListParams, ResourceExt},
    client::Client,
    runtime::controller::{Action, Controller},
    CustomResource, Resource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::debug;
use tracing::error;
use tracing::info;

use envconfig::Envconfig;
use serde_merge::omerge;

#[derive(Envconfig, Clone)]
pub struct TaskControllerConfig {
    #[envconfig(
        from = "EXECUTOR_IMAGE",
        default = "main.localhost:45373/ame-executor:latest"
    )]
    pub executor_image: String,

    #[envconfig(from = "NAMESPACE", default = "ame-system")]
    pub namespace: String,

    #[envconfig(from = "BUCKET", default = "ame")]
    pub bucket: String,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "Task",
    group = "ame.teainspace.com",
    version = "v1alpha1",
    namespaced
)]
#[kube(status = "TaskStatus", shortname = "task")]
#[serde(rename_all = "camelCase")]
pub struct TaskSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    // Runcommand defines the command AME will use to start this Task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runcommand: Option<String>,

    // Projectid defines which project this Task belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projectid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<TaskEnvVar>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    // Secrets that will be made available to the Task during execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<TaskSecret>>,

    // Pipeline defines a sequence of tasks to execute.
    // If a pipeline is specified the rest of the fields in this
    // specification are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline: Option<Vec<PipelineStep>>,

    // source defines where AME will pull the project from.
    // This can either be AME's own object storage or a git repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ProjectSource>,

    // Resources define what resources this Task requires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<BTreeMap<String, Quantity>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_type: Option<TaskType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_ref: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_ref: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_set: Option<Vec<String>>,
}

impl Task {
    pub fn new_gen_name(prefix: String, spec: TaskSpec) -> Self {
        Self {
            metadata: ObjectMeta {
                generate_name: Some(prefix),
                ..ObjectMeta::default()
            },
            spec,
            status: None,
        }
    }
}

impl TaskSpec {
    pub fn from_ref(TaskRef { name, project }: TaskRef) -> Self {
        let task_ref = if let Some(project) = project {
            format!("{project}.{name}")
        } else {
            name
        };

        Self {
            task_ref: Some(task_ref),
            ..Self::default()
        }
    }
}

#[derive(JsonSchema, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    PipEnv,
    Mlflow,
    Poetry,
}

/// The status object of `Task`
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Default)]
pub struct TaskStatus {
    pub phase: Option<TaskPhase>,
    pub reason: Option<String>,
    pub data_set_tasks: Option<BTreeMap<String, DataSetStatus>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Default)]
pub struct DataSetStatus {
    pub phase: TaskPhase,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq, Default)]
pub enum TaskPhase {
    Running,
    #[default]
    Pending,
    Failed,
    Succeeded,
    Error,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct TaskEnvVar {
    name: String,
    value: String,
}

impl From<&TaskEnvVar> for EnvVar {
    fn from(te: &TaskEnvVar) -> EnvVar {
        EnvVar {
            name: te.name.clone(),
            value: Some(te.value.clone()),
            ..EnvVar::default()
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct AmeSecret {
    pub name: String,
    pub envkey: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VaultSecret {
    pub vault_name: String,
    pub secret_key: String,
    pub secret_path: String,
    pub envkey: String,
}

impl VaultSecret {
    fn annotations(&self, service_account: String) -> BTreeMap<String, String> {
        let mut annotations = BTreeMap::new();

        annotations.insert(
            "vault.hashicorp.com/agent-inject".to_string(),
            "true".to_string(),
        );
        annotations.insert("vault.hashicorp.com/role".to_string(), service_account);
        annotations.insert(
            "vault.hashicorp.com/agent-inject-secret-config".to_string(),
            "internal/data/database/config".to_string(),
        );
        annotations.insert(
            "vault.hashicorp.com/agent-inject-template-config".to_string(),
            format!(
                "{{{{- with secret \"{}\" -}}}}
            export {}=\"{{{{ .Data.{} }}}}\"
          {{{{- end -}}}}",
                self.secret_path, self.envkey, self.secret_key
            ),
        );

        annotations
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum TaskSecret {
    AmeSecret(AmeSecret),
    VaultSecret(VaultSecret),
}

impl From<&AmeSecret> for EnvVar {
    fn from(ts: &AmeSecret) -> EnvVar {
        EnvVar {
            name: ts.envkey.clone(),
            value_from: Some(EnvVarSource {
                secret_key_ref: Some(SecretKeySelector {
                    key: "secret".to_string(),
                    name: Some(ts.name.clone()),
                    ..SecretKeySelector::default()
                }),
                ..EnvVarSource::default()
            }),
            ..EnvVar::default()
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct PipelineStep {
    taskname: String,
    runcommand: String,
    env: Vec<TaskEnvVar>,
    // Secrets that will be made available to the Task during execution.
    secrets: Vec<TaskSecret>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
pub struct ProjectSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gitrepository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gitreference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amestoragepath: Option<String>,
}

async fn resolve_template(
    name: &str,
    project_id: &str,
    projects: Api<project::Project>,
) -> Result<TaskSpec> {
    let Some(project) = projects
        .list(&ListParams::default())
        .await?
        .into_iter()
        .find(|p| p.spec.id == project_id) else {
        return Err(Error::MissingTemplate(
            project_id.to_string(),
            name.to_string(),
        ));
        };

    if let Some(template) = project.get_template(name) {
        Ok(template)
    } else {
        Err(Error::MissingTemplate(
            project_id.to_string(),
            name.to_string(),
        ))
    }
}

impl Task {
    fn common_wf_template(
        &self,
        name: String,
        scrict_src: String,
        volume_name: &str,
        addition_env: Option<Vec<EnvVar>>,
        config: &TaskControllerConfig,
    ) -> Result<WorkflowTemplate> {
        let required_env: Vec<EnvVar> = serde_json::from_value(json!([
            {
            "name":  "AWS_ACCESS_KEY_ID",
            "valueFrom": {
                "secretKeyRef":  {
                    "key": "MINIO_ROOT_USER",
                    "name": "ame-minio-secret",
                    "optional": false,
                }
            },
        },
        {
            "name":  "AWS_SECRET_ACCESS_KEY",
            "valueFrom": {
                "secretKeyRef":  {
                    "key": "MINIO_ROOT_PASSWORD",
                    "name": "ame-minio-secret",
                    "optional": false
                }
            },
        },
        {
            "name": "MLFLOW_TRACKING_URI",
            "value": "http://mlflow.default.svc.cluster.local:5000"
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

        let final_env = if let Some(vars) = addition_env {
            [required_env, vars].concat()
        } else {
            required_env
        };

        Ok(WorkflowTemplate {
            security_context: Some(serde_json::from_value(json!({
                "runAsUser": 1001,
                "fsGroup": 2000
            }
            ))?),
            script: Some(ArgoScriptTemplate {
                source: scrict_src,
                container: serde_json::from_value(json!(
                        {
                          "image": config.executor_image,
                          "command": ["bash"],
                          "volumeMounts": [{
                              "name": volume_name,
                              "mountPath": "/project",
                          }],
                          "env": final_env,
                          "resources": {},
                        }
                ))?,
            }),
            ..WorkflowTemplate::new(name)
        }
        .label("ame-task".to_string(), self.name_any())
        .clone())
    }

    fn generate_setup_template(
        &self,
        volume_name: &str,
        config: &TaskControllerConfig,
        data_sets: Option<Vec<DataSet>>,
    ) -> Result<WorkflowTemplate> {
        let data_set_download_command = data_sets.map(|data_sets| data_sets.into_iter().map(|ds| {
            format!("s3cmd --no-ssl --region eu-central-1 --host=$MINIO_URL --host-bucket=$MINIO_URL get --recursive s3://ame/tasks/{}/artifacts/{} ./

                ", local_name(ds.name), ds.path )
            }).collect::<String>());

        let project_pull_command = if let Some(ProjectSource {
            gitrepository: Some(ref repo),
            ..
        }) = self.spec.source
        {
            format!(
                "
                git clone {repo} repo

                cp -r repo/* .

                rm -rf repo

                ls
                "
            )
        } else {
            format!("s3cmd --no-ssl --region eu-central-1 --host=$MINIO_URL --host-bucket=$MINIO_URL get --recursive s3://{} ./", self.task_files_path())
        };
        let script_src = format!(
            "
            {}

            {project_pull_command}

                       echo \"0\" >> exit.status
                        ",
            data_set_download_command.unwrap_or_default()
        );
        self.common_wf_template("setup".to_string(), script_src, volume_name, None, config)
    }

    fn task_files_path(&self) -> String {
        format!("ame/tasks/{}/projectfiles/", self.name_any())
    }

    fn task_artifacts_path(&self) -> String {
        format!("ame/tasks/{}/artifacts/", self.name_any())
    }

    fn script_src(&self) -> String {
        let task_exec_cmd = if let Some(TaskType::Mlflow) = self.spec.task_type {
            "export PATH=$HOME/.pyenv/bin:$PATH

             unset AWS_SECRET_ACCESS_KEY

             unset AWS_ACCESS_KEY_ID

             mlflow run ."
                .to_string()
        } else if let Some(TaskType::Poetry) = self.spec.task_type {
            format!(
                "
                          poetry install

                          poetry run {}

                         save_artifacts {}
                ",
                self.spec
                    .runcommand
                    .clone()
                    .unwrap_or("missing command".to_string()), // TODO: handle missing commands
                self.task_artifacts_path()
            )
        } else {
            format!(
                "
                pipenv sync

                pipenv run {}

                save_artifacts {}",
                self.spec
                    .runcommand
                    .clone()
                    .unwrap_or("missing command".to_string()), // TODO: handle missing commands
                self.task_artifacts_path()
            )
        };

        let src = format!("
          set -e # It is important that the workflow exits with an error code if execute or save_artifacts fails, so AME can take action based on that information.

          {task_exec_cmd}

          echo \"0\" >> exit.status
            ",);

        if self.uses_vault() {
            format!(
                "
                source /vault/secrets/config

                {src}"
            )
        } else {
            src
        }
    }

    fn uses_vault(&self) -> bool {
        if let Some(ref secrets) = self.spec.secrets {
            secrets
                .iter()
                .any(|s| matches!(s, TaskSecret::VaultSecret(_)))
        } else {
            false
        }
    }

    fn vault_secrets(&self) -> Vec<&VaultSecret> {
        if let Some(secrets) = &self.spec.secrets {
            secrets
                .iter()
                .filter_map(|s| {
                    if let TaskSecret::VaultSecret(v) = s {
                        Some(v)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }

    fn generate_wf_template(
        &mut self,
        volume_name: &str,
        config: &TaskControllerConfig,
    ) -> Result<WorkflowTemplate> {
        let secret_env: Vec<EnvVar> = if let Some(secrets) = &self.spec.secrets {
            secrets
                .iter()
                .filter_map(|v| {
                    if let TaskSecret::AmeSecret(secret) = v {
                        Some(secret)
                    } else {
                        None
                    }
                })
                .map(EnvVar::from)
                .collect()
        } else {
            vec![]
        };

        let task_env: Vec<EnvVar> = if let Some(vars) = &self.spec.env {
            [secret_env, vars.iter().map(EnvVar::from).collect()].concat()
        } else {
            secret_env
        };

        let mut wf_template = WorkflowTemplate {
            pod_spec_patch: Some(self.generate_pod_spec_patch()?),
            ..self.common_wf_template(
                self.name_any(),
                self.script_src(),
                volume_name,
                Some(task_env),
                &TaskControllerConfig {
                    executor_image: self
                        .spec
                        .image
                        .clone()
                        .unwrap_or(config.clone().executor_image),
                    ..config.clone()
                },
            )?
        };

        if self.uses_vault() {
            let vault_secrets = self.vault_secrets();
            for vs in vault_secrets {
                wf_template = wf_template
                    .bulk_annotate(vs.annotations("ame-task".to_string()))
                    .clone();
            }
        }

        Ok(wf_template)
    }

    fn generate_pod_spec_patch(&self) -> Result<String> {
        Ok(format!(
            "{{\"containers\":[{{\"name\":\"main\", \"resources\":{{\"limits\":{}}}}}]}}",
            json!(self.spec.resources)
        ))
    }

    fn generate_workflow(
        &mut self,
        config: &TaskControllerConfig,
        _projects: Api<project::Project>,
        data_sets: Option<Vec<DataSet>>,
    ) -> Result<Workflow> {
        let volume_name = format!("{}-volume", self.name_any());
        let mut volume_resource_requirements = BTreeMap::new();
        volume_resource_requirements.insert("storage".to_string(), Quantity("50Gi".to_string()));
        let oref = if let Some(refe) = self.controller_owner_ref(&()) {
            refe
        } else {
            OwnerReference::default()
        };

        Ok(Workflow::default()
            .set_name(self.name_any())
            .set_service_account("ame-task".to_string())
            .label("ame-task".to_string(), self.name_any())
            .add_template(
                WorkflowTemplate::new("main".to_string())
                    .add_parallel_step(vec![WorkflowStep {
                        name: "setup".to_string(),
                        inline: Some(Box::new(self.generate_setup_template(
                            &volume_name,
                            config,
                            data_sets,
                        )?)),
                    }])
                    .add_parallel_step(vec![WorkflowStep {
                        name: "main".to_string(),
                        inline: Some(Box::new(self.generate_wf_template(&volume_name, config)?)),
                    }])
                    .clone(),
            )
            .add_volume_claim_template(PersistentVolumeClaim {
                metadata: ObjectMeta {
                    name: Some(volume_name.clone()),
                    ..ObjectMeta::default()
                },
                spec: Some(PersistentVolumeClaimSpec {
                    access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                    resources: Some(ResourceRequirements {
                        requests: Some(volume_resource_requirements),
                        limits: None,
                    }),
                    ..PersistentVolumeClaimSpec::default()
                }),

                // Note that it is important to create the equivalent of an empty struct here
                // and not just a None.
                // Otherwise the Workflow controller will disagree with AME's controller on
                // how an empty status should be specified.
                status: Some(PersistentVolumeClaimStatus::default()),
            })
            .add_owner_reference(oref)
            .clone())
    }

    async fn get_datasets(&self, projects: Api<project::Project>) -> Result<Vec<DataSet>> {
        let TaskSpec {
            data_set: Some(data_sets),
            ..
        } = self.spec.clone() else {
            return Ok(vec![])
        };

        let (remote_data_sets, _local_data_sets): (Vec<String>, Vec<String>) = data_sets
            .clone()
            .into_iter()
            .partition(|ds| ds.contains('.'));

        // TODO: this is very brittle
        let projects_ids: Vec<String> = remote_data_sets
            .clone()
            .into_iter()
            .map(|ds| ds.split('.').map(String::from).collect::<Vec<String>>()[0].clone())
            .collect();

        // TODO: this will break if datasets reference there own project.
        //

        debug!("looking for project vals");

        let project_vals: Vec<project::Project> = join_all(projects_ids.into_iter().map(|p| {
            let projects = projects.clone();
            async move {
                projects
                    .list(&ListParams::default())
                    .await?
                    .into_iter()
                    .find(|project| project.spec.id == p)
                    .ok_or(Error::MissingProject(p.to_string()))
            }
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<project::Project>>>()?;

        debug!("mapping remote datasets");

        // TODO: this is very brittle, we need more checks to ensure nothing went wrong.
        let data_sets: Vec<DataSet> = data_sets
            .clone()
            .into_iter()
            .map(|ref ds| {
                project_vals
                    .clone()
                    .into_iter()
                    .find_map(|pds| pds.get_data_set(ds.to_string()))
                    .ok_or(Error::MissingDataSets(ds.to_string()))
            })
            .collect::<Result<Vec<DataSet>>>()?;

        debug!("mapping local datasets");

        if data_sets.len() != data_sets.len() {
            return Err(Error::MissingDataSets(self.name_any())); // TODO: find a proper way of describing this error.
        }

        Ok(data_sets)
    }

    async fn get_data_set_tasks(&self, projects: Api<project::Project>) -> Result<Vec<Task>> {
        let TaskSpec {
            data_set: Some(data_sets),
            ..
        } = self.spec.clone() else {
            return Ok(vec![])
        };

        let (remote_data_sets, _local_data_sets): (Vec<String>, Vec<String>) = data_sets
            .clone()
            .into_iter()
            .partition(|ds| ds.contains('.'));

        // TODO: this is very brittle
        let projects_ids: Vec<String> = remote_data_sets
            .clone()
            .into_iter()
            .map(|ds| ds.split('.').map(String::from).collect::<Vec<String>>()[0].clone())
            .collect();

        // TODO: this will break if datasets reference there own project.
        //

        debug!("looking for project vals");

        let project_vals: Vec<project::Project> = join_all(projects_ids.into_iter().map(|p| {
            let projects = projects.clone();
            async move {
                projects
                    .list(&ListParams::default())
                    .await?
                    .into_iter()
                    .find(|project| project.spec.id == p)
                    .ok_or(Error::MissingProject(p.to_string()))
            }
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<project::Project>>>()?;

        debug!("mapping remote datasets");

        // TODO: this is very brittle, we need more checks to ensure nothing went wrong.
        let data_set_tasks: Vec<Task> = data_sets
            .clone()
            .into_iter()
            .map(|ref ds| {
                project_vals
                    .clone()
                    .into_iter()
                    .find_map(|pds| pds.generate_data_set_task(ds.to_string()))
                    .ok_or(Error::MissingDataSets(ds.to_string()))
            })
            .collect::<Result<Vec<Task>>>()?;

        debug!("mapping local datasets");

        if data_set_tasks.len() != data_sets.len() {
            return Err(Error::MissingDataSets(self.name_any())); // TODO: find a proper way of describing this error.
        }

        Ok(data_set_tasks)
    }

    async fn solve_template(&mut self, projects: Api<project::Project>) -> Result<()> {
        let TaskSpec {
            template_ref: Some(ref template_ref),
            ..
        } = self.spec else {
            return Ok(())
        };

        let (template_name, project_id) = {
            let vals: Vec<&str> = template_ref.split('.').collect();
            if vals.len() != 2 {
                return Err(Error::MissingTemplate("".to_string(), "".to_string()));
            }

            (vals[1], vals[0])
        };

        let template = resolve_template(template_name, project_id, projects).await?;

        self.spec = omerge(template, self.spec.clone())?;

        Ok(())
    }
}

#[derive(Clone)]
struct Context {
    client: Client,
    config: TaskControllerConfig,
}

async fn reconcile(task: Arc<Task>, ctx: Arc<Context>) -> Result<Action> {
    let tasks = Api::<Task>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let workflows = Api::<Workflow>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let projects = Api::<project::Project>::namespaced(ctx.client.clone(), &ctx.config.namespace);

    let mut task: Task = Task::clone(&task);

    let data_sets = task.clone().spec.data_set;

    let mut mut_task = task.clone();

    let Some(ref mut status) = mut_task.status_mut() else {
        let mut new_task = Task {
            metadata: task.metadata.clone(),
            spec: TaskSpec::default(),
            status: Some(TaskStatus {
                phase: Some(TaskPhase::Pending),
                reason: None,
                data_set_tasks: None,
            }),
        };

        new_task.metadata.managed_fields = None;


        tasks
            .patch_status(
                &task.name_any(),
                &PatchParams::apply("taskmanager.teainspace.com"),
                &kube::api::Patch::Apply(new_task),
            )
            .await?;

        return Ok(Action::requeue(Duration::from_secs(50)));
    };

    let task_phase = status.clone().phase.unwrap_or_default();
    if task_phase == TaskPhase::Failed || task_phase == TaskPhase::Succeeded {
        return Ok(Action::requeue(Duration::from_secs(50)));
    }

    if let Some(_data_sets) = data_sets.as_ref() {
        debug!("reconciling datasets");

        // TODO: how do we make this idempotent?
        let data_sets = task.get_data_set_tasks(projects.clone()).await?;

        let data_set_statuses: Vec<(String, DataSetStatus)> =
            join_all(data_sets.iter().map(|data_task: &Task| async {
                let mut data_task = data_task.to_owned();

                data_task.metadata = add_owner_reference(
                    data_task.metadata.clone(),
                    task.controller_owner_ref(&()).unwrap_or_default(),
                );

                let data_task: Task = tasks
                    .patch(
                        &data_task.name_any(),
                        &PatchParams::apply("ame-controller"),
                        &Patch::Apply(&data_task),
                    )
                    .await?;

                Ok((
                    data_task.name_any(),
                    DataSetStatus {
                        phase: data_task
                            .status
                            .and_then(|status| status.phase)
                            .unwrap_or_default(),
                    },
                )) as crate::Result<(String, DataSetStatus)>
            }))
            .await
            .into_iter()
            .collect::<Result<_>>()?;

        let mut data_set_status_map: BTreeMap<String, DataSetStatus> = BTreeMap::new();

        for (name, status) in data_set_statuses.clone() {
            data_set_status_map.insert(name, status);
        }

        // TODO: what if status is None?
        match &mut status.data_set_tasks {
            None => {
                status.data_set_tasks = Some(data_set_status_map);
            }
            Some(data_set_tasks) => {
                for (k, v) in data_set_status_map.into_iter() {
                    data_set_tasks.insert(k, v);
                }
            }
        };

        if data_set_statuses
            .clone()
            .into_iter()
            .any(|(_name, dst)| dst.phase == TaskPhase::Failed)
        {
            status.phase = Some(TaskPhase::Failed);
            status.reason = Some("one or more data sources have failed".to_string());

            task.meta_mut().managed_fields = None;

            let res = tasks
                .patch_status(
                    &task.name_any(),
                    &PatchParams::apply("taskmanager.teainspace.com"),
                    &kube::api::Patch::Apply(task),
                )
                .await;

            info!("one or more data sources have failed");

            if let Err(e) = res {
                return Err(Error::KubeApiError(e));
            }

            return Ok(Action::requeue(Duration::from_secs(50)));
        }

        if data_set_statuses
            .into_iter()
            .any(|(_, dst)| dst.phase != TaskPhase::Succeeded)
        {
            status.phase = Some(TaskPhase::Pending);
            status.reason = Some("one or more data sources are not ready".to_string());
            // TODO: should we really be setting managed_fields to None?
            task.meta_mut().managed_fields = None;
            let res = tasks
                .patch_status(
                    &task.name_any(),
                    &PatchParams::apply("taskmanager.teainspace.com"),
                    &kube::api::Patch::Apply(task),
                )
                .await;

            info!("one or more data sources are not ready");

            if let Err(e) = res {
                return Err(Error::KubeApiError(e));
            }

            return Ok(Action::requeue(Duration::from_secs(50)));
        }
    }

    task.solve_template(projects.clone()).await?;

    let data_sets = task.get_datasets(projects.clone()).await?;

    let correct_wf = task.generate_workflow(&ctx.config, projects, Some(data_sets))?;
    let wf = match workflows
        .patch(
            &correct_wf.name_any(),
            &PatchParams::apply("taskmanager.teainspace.com"),
            &kube::api::Patch::Apply(&correct_wf),
        )
        .await
    {
        Ok(wf) => wf,
        Err(kube::Error::Api(ErrorResponse { code: 409, .. })) => {
            workflows
                .patch(
                    &correct_wf.name_any(),
                    &PatchParams::apply("taskmanager.teainspace.com").force(),
                    &kube::api::Patch::Apply(correct_wf),
                )
                .await?
        }
        Err(e) => {
            let wf = workflows.get(&correct_wf.name_any()).await?;
            similar_asserts::assert_eq!(wf.spec, correct_wf.spec);
            return Err(e)?;
        }
    };

    if let Some(wf_status) = &wf.status {
        let correct_task_phase = match wf_status.phase {
            WorkflowPhase::Running => TaskPhase::Running,
            WorkflowPhase::Pending => TaskPhase::Pending,
            WorkflowPhase::Succeeded => TaskPhase::Succeeded,
            WorkflowPhase::Failed => TaskPhase::Failed,
            WorkflowPhase::Error => TaskPhase::Error,
        };

        if correct_task_phase != task_phase {
            let mut new_task = task.clone();
            let original_status = new_task.status.unwrap_or(TaskStatus::default());

            new_task.metadata.managed_fields = None;
            new_task.status = Some(TaskStatus {
                phase: Some(correct_task_phase),
                ..original_status
            });

            let res = tasks
                .patch_status(
                    &new_task.name_any(),
                    &PatchParams::apply("taskmanager.teainspace.com"),
                    &kube::api::Patch::Apply(new_task),
                )
                .await;

            match res {
                Ok(_) => println!("Patched status for task: {} ", task.name_any()),
                Err(e) => {
                    return Err(Error::KubeApiError(e));
                }
            };
        }
    }

    Ok(Action::requeue(Duration::from_secs(50)))
}

fn error_policy(task: Arc<Task>, error: &Error, _ctx: Arc<Context>) -> Action {
    error!("error: {}, for task: {}", error, task.name_any());
    Action::requeue(Duration::from_secs(5))
}

pub async fn start_task_controller(config: TaskControllerConfig) -> BoxFuture<'static, ()> {
    let client = Client::try_default().await.expect("Failed to create a K8S client, is the controller running in an environment with access to cluster credentials?");
    let context = Arc::new(Context {
        client: client.clone(),
        config,
    });

    let tasks = Api::<Task>::namespaced(client.clone(), &context.config.namespace);
    let _ = tasks
        .list(&ListParams::default())
        .await
        .expect("Is the CRD installed?");

    let workflows = Api::<Workflow>::namespaced(client, &context.config.namespace);

    Controller::new(tasks.clone(), ListParams::default())
        .owns(workflows, ListParams::default())
        .owns(tasks, ListParams::default())
        .run(reconcile, error_policy, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .boxed()
}

#[cfg(test)]
mod test {

    use super::*;
    use either::Either;
    use insta;
    use kube::api::{DeleteParams, PostParams, WatchEvent};
    use serial_test::serial;

    fn gen_test_config() -> TaskControllerConfig {
        // It is important to explicitly set the struct values, so the environment cannot impact
        // test runs.
        TaskControllerConfig {
            namespace: String::from("default"),
            executor_image: "ghcr.io/teainspace/ame/ame-executor:0.0.3".to_string(),
            bucket: String::from("ame"),
        }
    }

    /// Prepare a cluster for tests, under the assumptions that the `just setup_cluster` recipe has been run successfully.
    /// This implies that all required custom resource definitions are installed in the cluster.
    /// This function will generate clients and clear all Task and `Workflow` objects in the cluster.
    async fn setup_cluster(
    ) -> Result<(TaskControllerConfig, Api<Task>, Api<Workflow>), Box<dyn std::error::Error>> {
        let config = gen_test_config();
        let client = Client::try_default().await?;
        let tasks = Api::<Task>::namespaced(client.clone(), &config.namespace);
        let workflows = Api::<Workflow>::namespaced(client.clone(), &config.namespace);

        let dp = DeleteParams::default();
        let lp = ListParams::default();

        match tasks.delete_collection(&dp, &lp).await? {
            Either::Left(_) => {
                while !tasks.list(&lp).await?.items.is_empty() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            Either::Right(status) => {
                println!("Deleted collection of tasks: {status:?}");
            }
        };

        match workflows.delete_collection(&dp, &lp).await? {
            Either::Left(_) => {
                while !workflows.list(&lp).await?.items.is_empty() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            Either::Right(status) => {
                println!("Deleted collection of tasks: {status:?}");
            }
        };

        // TODO: How do we expose controller errors in tests?
        let controller = start_task_controller(config.clone()).await;
        tokio::spawn(controller);

        Ok((config, tasks, workflows))
    }

    #[tokio::test]
    #[serial]
    async fn can_create_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tasks, workflows) = setup_cluster().await?;

        let all_wfs = workflows.list(&ListParams::default()).await.unwrap();
        assert_eq!(all_wfs.items.len(), 0);

        let t: Task = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Task",
            "metadata": { "generateName": "training" },
            "spec": {
                "runcommand": "python train.py",
                "projectid": "myproject",
            }
        }))
        .unwrap();

        let pp = PostParams::default();
        let task_in_cluster = tasks.create(&pp, &t).await?;

        let lp = ListParams::default()
            .labels(&format!("ame-task={}", task_in_cluster.name_any()))
            .timeout(5);

        let mut stream = workflows.watch(&lp, "0").await?.boxed();
        while let Some(status) = stream.next().await {
            if let WatchEvent::Added(_) = status? {
                return Ok(());
            }
        }

        panic!("Did not find workflow");
    }

    #[tokio::test]
    #[serial]
    async fn controller_adds_status_to_new_task() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tasks, _) = setup_cluster().await?;

        let t: Task = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Task",
            "metadata": { "generateName": "training" },
            "spec": {
                "runcommand": "python train.py",
                "projectid": "myproject",
            }
        }))
        .unwrap();

        let pp = PostParams::default();
        let task_in_cluster = tasks.create(&pp, &t).await?;

        let lp = ListParams::default()
            .fields(&format!("metadata.name={}", task_in_cluster.name_any()))
            .timeout(5);

        let mut stream = tasks.watch(&lp, "0").await?.boxed();
        while let Some(status) = stream.next().await {
            if let WatchEvent::Modified(t) = status? {
                if let Some(status) = t.status {
                    assert_eq!(status.phase, Some(TaskPhase::Pending));
                    return Ok(());
                }
            }
        }

        panic!("Did not add status");
    }

    // TODO: Should we test some fields explicitly?
    #[tokio::test]
    async fn task_can_generate_workflow() {
        let mut t: Task = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Task",
            "metadata": { "name": "training" },
            "spec": {
                "runcommand": "python train.py",
                "projectid": "myproject",
                "env": [
                {
                    "name": "VAR1",
                    "value": "val1"
                },
                {
                    "name": "VAR2",
                    "value": "val2"
                },
                {
                    "name": "VAR3",
                    "value": "val3"
                }
                ],
                "secrets": [
                {
                    "AmeSecret" : {
                    "name": "secret1",
                    "envkey": "KEY1"
                    }
                },
                {
                    "AmeSecret" : {
                    "name": "secret2",
                    "envkey": "KEY2"
                    }
                },
                ]
            }
        }))
        .unwrap();

        let client = Client::try_default().await.unwrap();
        let projects = Api::<project::Project>::default_namespaced(client);
        let wf: Workflow = t
            .generate_workflow(
                &gen_test_config(),
                projects,
                Some(vec![DataSet {
                    name: "mydataset".to_string(),
                    task: TaskSpec::default(),
                    path: "path_to_data".to_string(),
                }]),
            )
            .unwrap();

        insta::assert_yaml_snapshot!(&wf);
    }

    #[tokio::test]
    async fn task_can_override_workflow_image() {
        let mut t: Task = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Task",
            "metadata": { "name": "training" },
            "spec": {
                "runcommand": "python train.py",
                "projectid": "myproject",
                "env": [
                {
                    "name": "VAR1",
                    "value": "val1"
                },
                {
                    "name": "VAR2",
                    "value": "val2"
                }
                ],
                "image": "very-different-image",
                "secrets": [
                {
                    "AmeSecret" : {
                    "name": "secret1",
                    "envkey": "KEY1"
                    }
                },
                {
                    "AmeSecret" : {
                    "name": "secret2",
                    "envkey": "KEY2"
                    }
                },
                ]
            }
        }))
        .unwrap();

        let client = Client::try_default().await.unwrap();
        let projects = Api::<project::Project>::default_namespaced(client);
        let wf: Workflow = t
            .generate_workflow(&gen_test_config(), projects, None)
            .unwrap();

        insta::assert_yaml_snapshot!(&wf);
    }

    #[tokio::test]
    async fn task_can_have_git_src() {
        let mut t: Task = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Task",
            "metadata": { "name": "training" },
            "spec": {
                "runcommand": "python train.py",
                "projectid": "myproject",
                "env": [
                {
                    "name": "VAR1",
                    "value": "val1"
                },
                {
                    "name": "VAR2",
                    "value": "val2"
                }
                ],
                "secrets": [
                {
                    "AmeSecret" : {
                    "name": "secret1",
                    "envkey": "KEY1"
                    }
                },
                {
                    "AmeSecret" : {
                    "name": "secret2",
                    "envkey": "KEY2"
                    }
                },
                ],
                "source": {
                    "gitrepository": "gitrepo",
                },
            }
        }))
        .unwrap();

        let client = Client::try_default().await.unwrap();
        let projects = Api::<project::Project>::default_namespaced(client);
        let wf: Workflow = t
            .generate_workflow(&gen_test_config(), projects, None)
            .unwrap();

        insta::assert_yaml_snapshot!(&wf);
    }

    #[tokio::test]
    async fn task_can_have_vault_secret() {
        let mut t: Task = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Task",
            "metadata": { "name": "training" },
            "spec": {
                "runcommand": "python train.py",
                "projectid": "myproject",
                "env": [
                {
                    "name": "VAR1",
                    "value": "val1"
                },
                {
                    "name": "VAR2",
                    "value": "val2"
                }
                ],
                "secrets": [
                {
                    "VaultSecret" : {
                    "vaultName": "myvault",
                    "secretKey": "mysecret",
                    "secretPath": "/my/secret/path",
                    "envkey": "KEY1",
                    }
                },
                {
                    "AmeSecret" : {
                    "name": "secret2",
                    "envkey": "KEY2"
                    }
                },
                ],
            }
        }))
        .unwrap();

        let client = Client::try_default().await.unwrap();
        let projects = Api::<project::Project>::default_namespaced(client);
        let wf: Workflow = t
            .generate_workflow(&gen_test_config(), projects, None)
            .unwrap();

        insta::assert_yaml_snapshot!(&wf);
    }

    #[tokio::test]
    #[serial]
    async fn can_correct_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let (_, tasks, workflows) = setup_cluster().await?;

        let all_wfs = workflows.list(&ListParams::default()).await.unwrap();
        assert_eq!(all_wfs.items.len(), 0);

        let all_tasks = tasks.list(&ListParams::default()).await.unwrap();
        assert_eq!(all_tasks.items.len(), 0);

        let t: Task = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "Task",
            "metadata": { "generateName": "training" },
            "spec": {
                "runcommand": "python train.py",
                "projectid": "myproject",
            }
        }))
        .unwrap();

        let task_in_cluster = tasks.create(&PostParams::default(), &t).await?;

        let lp = ListParams::default()
            .labels(&format!("ame-task={}", task_in_cluster.name_any()))
            .timeout(5);

        let mut correct_labels = BTreeMap::new();
        let mut stream = workflows.watch(&lp, "0").await?.boxed();
        while let Some(e) = stream.next().await {
            if let WatchEvent::Added(wf) = e? {
                correct_labels = wf.metadata.labels.clone().unwrap();
                let mut labels = wf.metadata.labels.clone().unwrap();
                labels.insert("ame-task".to_string(), "test".to_string());
                workflows
                    .patch(
                        &wf.name_any(),
                        &PatchParams::apply("taskmanager.teainspace.com"),
                        &kube::api::Patch::Apply(Workflow {
                            metadata: ObjectMeta {
                                labels: Some(labels),
                                managed_fields: None,
                                ..wf.metadata
                            },
                            ..wf
                        }),
                    )
                    .await?;

                break;
            }
        }

        // TODO: make the event streams in this test run in parallel so
        // so we can track that the controller produces the correct events without
        // relying on timing to workout.

        // TODO: Determine why we received Added and not Modified events.
        let mut stream = workflows.watch(&lp, "0").await?.boxed();
        while let Some(wf) = stream.next().await {
            match wf {
                Ok(WatchEvent::Added(wf)) => {
                    println!("wf labels {:?} ", wf.metadata.labels);
                    assert_eq!(wf.metadata.labels.unwrap(), correct_labels);
                    return Ok(());
                }

                e => println!("event: {e:?}"),
            }
        }

        panic!("workflow was not corrected")
    }
}
