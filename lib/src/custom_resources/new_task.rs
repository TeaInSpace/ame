use std::collections::BTreeMap;

use super::{
    argo::{Workflow, WorkflowBuilder, WorkflowTemplateBuilder},
    common::parent_project,
    data_set::DataSet,
    project::{add_owner_reference, Project},
    secrets::SecretReference,
};
use crate::{
    custom_resources::{find_project, task_ctrl::resolve_data_set_path},
    error::AmeError,
    grpc::{task_status, ArtifactCfg, TemplateRef},
    Result,
};
use k8s_openapi::apimachinery::pkg::{api::resource::Quantity, apis::meta::v1::OwnerReference};
use kube::{core::ObjectMeta, Api, CustomResource, Resource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_merge::omerge;
use tracing::debug;

use crate::grpc::{TaskCfg, TaskPhasePending, TaskStatus};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "Task",
    group = "ame.teainspace.com",
    version = "v1alpha1",
    namespaced
)]
#[kube(status = "TaskStatus", shortname = "tk")]
#[serde(rename_all = "camelCase")]
pub struct TaskSpec {
    #[serde(flatten)]
    pub cfg: TaskCfg,
    pub deletion_approved: bool,
    pub source: Option<ProjectSource>,
    pub project: Option<String>,
}

impl TaskStatus {
    pub fn pending() -> Self {
        TaskStatus {
            phase: Some(task_status::Phase::pending()),
        }
    }
}

impl task_status::Phase {
    pub fn pending() -> Self {
        Self::Pending(TaskPhasePending {})
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProjectSource {
    Git {
        repository: String,
        reference: String,
        user_name: String,
        secret: Option<SecretReference>,
    },
    Ame {
        path: String,
    },
}

impl ProjectSource {
    pub fn from_public_git_repo(repository: String) -> Self {
        ProjectSource::Git {
            repository,
            reference: "".to_string(),
            user_name: "".to_string(),
            secret: None,
        }
    }

    fn command(&self) -> String {
        match self {
            ProjectSource::Git {
                repository,
                reference: _,
                user_name: _,
                secret: None,
            } => {
                format!(
                    "
                    
                git clone {repository} repo

                cd repo

                git fetch origin

                git checkout origin/reference

                cd ..

                cp -r repo/* .

                rm -rf repo

                "
                )
            }
            ProjectSource::Ame { path } => {
                format!("s3cmd --no-ssl --region eu-central-1 --host=$MINIO_URL --host-bucket=$MINIO_URL get --recursive s3://{path} ./" )
            }
            _ => todo!("add tests for other git sources"),
        }
    }
}

pub fn build_workflow(task: Task, ctx: TaskContext) -> Result<Workflow> {
    debug!("building task with context: {:?}", ctx);

    let mut wf_builder = WorkflowBuilder::new(task.name_any(), ctx.service_account.clone());

    let mut volume_resource_requirements = BTreeMap::new();
    volume_resource_requirements.insert("storage".to_string(), Quantity("50Gi".to_string()));

    wf_builder.add_volume(task.name_any(), volume_resource_requirements);

    if let Some(oref) = task.controller_owner_ref(&()) {
        wf_builder.add_owner_reference(oref);
    };

    let setup_template =
        WorkflowTemplateBuilder::new(&ctx, task.load_command(&ctx)?, "setup".to_string())?
            .build(&task)?;

    wf_builder.add_template(setup_template);

    let main_template =
        WorkflowTemplateBuilder::new(&ctx, task.exec_command()?, task.name_any())?.build(&task)?;
    wf_builder.add_template(main_template);

    if task.should_save_artifacts() {
        let artifact_save_template = WorkflowTemplateBuilder::new(
            &ctx,
            task.artifact_save_command()?,
            "saveartifacts".to_string(),
        )?
        .build(&task)?;
        wf_builder.add_template(artifact_save_template);
    }

    wf_builder.build()
}

impl Task {
    pub fn approve_deletion_patch() -> Self {
        Task {
            metadata: ObjectMeta::default(),
            spec: TaskSpec {
                deletion_approved: true,
                ..TaskSpec::default()
            },
            status: None,
        }
    }

    pub fn project(&self) -> Result<String> {
        parent_project(self.owner_references().to_vec())
    }

    fn should_save_artifacts(&self) -> bool {
        self.spec.cfg.artifact_cfg.is_some()
    }

    fn artifact_save_command(&self) -> Result<String> {
        match self.spec.cfg.artifact_cfg {
            Some(ArtifactCfg{ save_changed_files, ..}) if save_changed_files => Ok(format!("save_artifacts {}", self.artifact_path()?)),
            Some(ArtifactCfg { ref paths, .. }) => Ok(paths.iter().map(|p| format!("s3cmd --no-ssl --region us-east-1 --host=$MINIO_URL --host-bucket=$MINIO_URL put {p} s3://$ARTIFACT_STORAGE_PATH{p}") ).collect::<Vec<String>>().join("\n\n")),
            None => Err(AmeError::EmptyArtifactCfg(self.spec.cfg.name.clone().unwrap_or_default()))
        }
    }

    fn artifact_path(&self) -> Result<String> {
        if let Some(ref name) = self.metadata.name {
            Ok(format!("ame/tasks/{name}/artifacts/"))
        } else {
            Err(AmeError::MissingName)
        }
    }

    fn load_command(&self, ctx: &TaskContext) -> Result<String> {
        let mut cmd = String::new();

        for ds in ctx.required_data_sets.iter() {
            cmd = format!("cmd \n\n s3cmd --no-ssl --region eu-central-1 --host=$MINIO_URL --host-bucket=$MINIO_URL get --recursive s3://{} ./

                \n\n", resolve_data_set_path(ds.clone())?)
        }

        let parent_project = parent_project(self.owner_references().to_vec())?;

        let load_cmd = if let Some(ref source) = self.spec.source {
            source.command()
        } else {
            ProjectSource::Ame {
                path: self.project_dir_path(parent_project),
            }
            .command()
        };

        Ok(format!("{cmd} \n\n {load_cmd}"))
    }

    fn project_dir_path(&self, parent_project: String) -> String {
        format!("ame/tasks/{parent_project}/projectfiles/")
    }

    fn exec_command(&self) -> Result<String> {
        self.spec
            .cfg
            .executor
            .as_ref()
            .map(|executor| executor.command())
            .ok_or(AmeError::MissingExecutor(self.name_any()))
    }
}

#[derive(Clone)]
pub struct TaskBuilder {
    task: Task,
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskBuilder {
    pub fn new() -> Self {
        Self {
            task: Task {
                metadata: ObjectMeta::default(),
                spec: TaskSpec::default(),
                status: None,
            },
        }
    }

    pub fn set_project(&mut self, name: String) -> &mut Self {
        self.task.spec.project = Some(name);
        self
    }

    pub fn from_cfg(cfg: TaskCfg) -> Self {
        Self {
            task: Task {
                metadata: ObjectMeta::default(),
                spec: TaskSpec::from(cfg),
                status: None,
            },
        }
    }

    pub fn set_name_prefix(&mut self, prefix: String) -> &mut Self {
        self.task.metadata.generate_name = Some(prefix);
        self
    }

    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.task.metadata.name = Some(name);
        self
    }

    pub fn set_project_src(&mut self, src: ProjectSource) -> &mut Self {
        self.task.spec.source = Some(src);

        self
    }

    pub fn set_model_version(&mut self, src: String) -> &mut Self {
        self.task
            .annotations_mut()
            .insert("model_source".to_string(), src);
        self
    }

    pub fn build(mut self) -> Task {
        if self.task.name_any().is_empty() {
            let prefix = self
                .task
                .spec
                .cfg
                .name
                .clone()
                .unwrap_or("task".to_string());
            self.set_name_prefix(prefix);
        }

        self.task.clone()
    }

    pub fn add_owner_reference(&mut self, owner_ref: OwnerReference) -> &mut Self {
        self.task.metadata = add_owner_reference(self.task.meta().clone(), owner_ref);

        self
    }
}

impl From<TaskCfg> for TaskSpec {
    fn from(cfg: TaskCfg) -> Self {
        TaskSpec {
            cfg,
            ..TaskSpec::default()
        }
    }
}

pub async fn resolve_task_templates(
    task: Task,
    project: Project,
    projects: Api<Project>,
) -> Result<Task> {
    let Some(ref template_ref) = task.spec.cfg.from_template else {
        return Ok(task);
    };

    let template = match template_ref {
        TemplateRef {
            ref name,
            project: None,
        } => project.get_template(name).ok_or(AmeError::MissingTemplate(
            name.clone(),
            project.spec.cfg.name,
        ))?,
        TemplateRef {
            name,
            project: Some(project_name),
        } => find_project(projects, project_name.clone(), "".to_string())
            .await
            .map_err(|_| AmeError::MissingProject(0))?
            .get_template(name)
            .ok_or(AmeError::MissingTemplate(
                name.clone(),
                project.spec.cfg.name,
            ))?,
    };

    debug!("found template {:?}", template);

    let task_spec = TaskSpec {
        cfg: omerge(template, task.spec.cfg)?,
        ..task.spec
    };

    Ok(Task {
        spec: task_spec,
        ..task
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    pub executor_image: String,
    pub task_volume: String,
    pub required_data_sets: Vec<DataSet>,
    pub service_account: String,
}

#[cfg(test)]
mod test {

    use kube::core::ObjectMeta;

    use crate::grpc::{
        secret::Variant, task_cfg::Executor, AmeSecretVariant, EnvVar, PoetryExecutor, Secret,
        TaskPhaseRunning, TaskRef,
    };

    use super::*;

    #[test]
    fn snap_shot_task_yaml() -> Result<()> {
        let mut resources = BTreeMap::new();
        resources.insert("cpu".to_string(), "2".to_string());

        let task = Task {
            metadata: ObjectMeta {
                name: Some("mytask".to_string()),
                ..ObjectMeta::default()
            },

            spec: TaskSpec {
                cfg: TaskCfg {
                    name: Some("mytask".to_string()),
                    task_ref: Some(TaskRef {
                        name: "othertask".to_string(),
                        project: None,
                    }),
                    executor: None,
                    resources,
                    data_sets: Vec::new(),
                    from_template: None,
                    artifact_cfg: None,
                    triggers: None,
                    env: vec![EnvVar {
                        key: "SOME_VAR".to_string(),
                        val: "someval".to_string(),
                    }],
                    secrets: vec![Secret {
                        variant: Some(Variant::Ame(AmeSecretVariant {
                            key: "secretkey".to_string(),
                            inject_as: "MY_SECRET".to_string(),
                        })),
                    }],
                },
                source: Some(ProjectSource::Ame {
                    path: "test".to_string(),
                }),
                deletion_approved: false,
                project: None,
            },
            status: Some(TaskStatus {
                phase: Some(task_status::Phase::Running(TaskPhaseRunning {
                    workflow_name: "someinfo".to_string(),
                })),
            }),
        };

        insta::assert_yaml_snapshot!(&task);

        Ok(())
    }
    #[test]
    fn snap_shot_workflow_yaml() -> Result<()> {
        let mut resources = BTreeMap::new();
        resources.insert("cpu".to_string(), "2".to_string());
        resources.insert("memory".to_string(), "2Gi".to_string());
        let task = Task {
            metadata: ObjectMeta {
                name: Some("mytask".to_string()),
                owner_references: Some(vec![OwnerReference {
                    block_owner_deletion: None,
                    api_version: "sfsdd".to_string(),
                    controller: None,
                    kind: "Project".to_string(),
                    name: "parentproject343".to_string(),
                    uid: "sdsfdsf".to_string(),
                }]),
                ..ObjectMeta::default()
            },

            spec: TaskSpec {
                cfg: TaskCfg {
                    name: Some("mytask".to_string()),
                    task_ref: Some(TaskRef {
                        name: "othertask".to_string(),
                        project: None,
                    }),
                    executor: Some(Executor::Poetry(PoetryExecutor {
                        python_version: "3.11".to_string(),
                        command: "python train.py".to_string(),
                    })),
                    data_sets: Vec::new(),
                    resources,
                    from_template: None,
                    artifact_cfg: Some(ArtifactCfg {
                        save_changed_files: true,
                        paths: vec![],
                    }),
                    triggers: None,
                    env: vec![EnvVar {
                        key: "SOME_VAR".to_string(),
                        val: "someval".to_string(),
                    }],
                    secrets: vec![Secret {
                        variant: Some(Variant::Ame(AmeSecretVariant {
                            key: "secretkey".to_string(),
                            inject_as: "MY_SECRET".to_string(),
                        })),
                    }],
                },
                source: Some(ProjectSource::Ame {
                    path: "test".to_string(),
                }),
                deletion_approved: false,
                project: None,
            },
            status: Some(TaskStatus {
                phase: Some(task_status::Phase::Running(TaskPhaseRunning {
                    workflow_name: "someinfo".to_string(),
                })),
            }),
        };

        // TODO: "create data set for testing ");

        let task_ctx = TaskContext {
            executor_image: "myimage".to_string(),
            task_volume: "myvolume".to_string(),
            required_data_sets: vec![],
            service_account: "ame-task".to_string(),
        };

        insta::assert_yaml_snapshot!(build_workflow(task, task_ctx)?);

        Ok(())
    }
}
