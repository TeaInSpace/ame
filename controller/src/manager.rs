use crate::argo::ArgoScriptTemplate;
use crate::argo::WorkflowStep;
use crate::argo::WorkflowTemplate;
use crate::Workflow;
use crate::WorkflowPhase;
use crate::{Error, Result};
use futures::future::BoxFuture;
use futures::FutureExt;
use futures::StreamExt;

use k8s_openapi::api::core::v1::EnvVar;
use k8s_openapi::api::core::v1::EnvVarSource;

use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use k8s_openapi::api::core::v1::PersistentVolumeClaimSpec;
use k8s_openapi::api::core::v1::ResourceRequirements;

use k8s_openapi::api::core::v1::SecretKeySelector;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube::api::PatchParams;
use kube::core::ObjectMeta;
use kube::{
    api::{Api, ListParams, PostParams, ResourceExt},
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

use envconfig::Envconfig;

#[derive(Envconfig, Clone)]
pub struct TaskControllerConfig {
    #[envconfig(
        from = "EXECUTOR_IMAGE",
        default = "ghcr.io/teainspace/ame/ame-executor:0.0.4"
    )]
    pub executor_image: String,

    #[envconfig(from = "NAMESPACE", default = "ame-system")]
    pub namespace: String,
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "Task",
    group = "ame.teainspace.com",
    version = "v1alpha1",
    namespaced
)]
#[kube(status = "TaskStatus", shortname = "task")]
pub struct TaskSpec {
    // Runcommand defines the command AME will use to start this Task.
    pub runcommand: String,

    // Projectid defines which project this Task belongs to.
    pub projectid: String,
    pub env: Option<Vec<TaskEnvVar>>,
    pub image: Option<String>,
    // Secrets that will be made available to the Task during execution.
    pub secrets: Option<Vec<TaskSecret>>,

    // Pipeline defines a sequence of tasks to execute.
    // If a pipeline is specified the rest of the fields in this
    // specification are ignored.
    pub pipeline: Option<Vec<PipelineStep>>,

    // source defines where AME will pull the project from.
    // This can either be AME's own object storage or a git repository.
    pub source: Option<ProjectSource>,

    // Resources define what resources this Task requires.
    pub resources: Option<BTreeMap<String, Quantity>>,
}

/// The status object of `Task`
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
pub struct TaskStatus {
    phase: Option<TaskPhase>,
    reason: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq, Eq)]
pub enum TaskPhase {
    Running,
    Pending,
    Failed,
    Succeeded,
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
pub struct TaskSecret {
    name: String,
    envkey: String,
}

impl From<&TaskSecret> for EnvVar {
    fn from(ts: &TaskSecret) -> EnvVar {
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

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct ProjectSource {
    gitrepository: Option<String>,
    gitreference: Option<String>,
    amestoragepath: Option<String>,
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
            "value": "minio",
        },
        {
            "name":  "AWS_SECRET_ACCESS_KEY",
            "value": "minio123",
        },
        {
            "name":  "MINIO_URL",
            "value": "http://ame-minio.ame-system.svc.cluster.local:9000",
        },
        {
            "name":  "TASK_DIRECTORY",
            "value": format!("ameprojectstorage/{}",  self.spec.projectid),
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
            securitycontext: Some(serde_json::from_value(json!({
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
    ) -> Result<WorkflowTemplate> {
        let project_pull_command = "s3cmd --no-ssl --region us-east-1 --host=$MINIO_URL --host-bucket=$MINIO_URL get --recursive s3://$TASK_DIRECTORY ./";
        let script_src = format!(
            "
                       {}
                       echo \"0\" >> exit.status
                        ",
            project_pull_command
        );
        self.common_wf_template("setup".to_string(), script_src, volume_name, None, config)
    }

    fn generate_wf_template(
        &self,
        volume_name: &str,
        config: &TaskControllerConfig,
    ) -> Result<WorkflowTemplate> {
        let scrict_src = format!("
          export TASK_DIRECTORY=ameprojectstorage/{}

          cd \"./{}\" 

          set -e # It is important that the workflow exits with an error code if execute or save_artifacts fails, so AME can take action based on that information.

          execute \"{}\" 
          
          save_artifacts \"ameprojectstorage/{}/artifacts/\"

          echo \"0\" >> exit.status
            ", self.spec.projectid, self.spec.projectid, self.spec.runcommand, self.name_any());

        let secret_env: Vec<EnvVar> = if let Some(secrets) = &self.spec.secrets {
            secrets.iter().map(EnvVar::from).collect()
        } else {
            vec![]
        };

        let task_env: Vec<EnvVar> = if let Some(vars) = &self.spec.env {
            [secret_env, vars.iter().map(EnvVar::from).collect()].concat()
        } else {
            secret_env
        };

        Ok(WorkflowTemplate {
            podspecpatch: Some(self.generate_pod_spec_patch()?),
            ..self.common_wf_template(
                self.name_any(),
                scrict_src,
                volume_name,
                Some(task_env),
                &TaskControllerConfig { executor_image: self.spec.image.clone().unwrap_or(config.clone().executor_image), ..config.clone() },
            )?
        })
    }

    fn generate_pod_spec_patch(&self) -> Result<String> {
        Ok(format!(
            "{{\"containers\":[{{\"name\":\"main\", \"resources\":{{\"limits\":{}}}}}]}}",
            json!(self.spec.resources)
        ))
    }

    fn generate_workflow(&self, config: &TaskControllerConfig) -> Result<Workflow> {
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
            .label("task".to_string(), self.name_any())
            .add_template(
                WorkflowTemplate::new("main".to_string())
                    .add_parallel_step(vec![WorkflowStep {
                        name: "setup".to_string(),
                        inline: Some(Box::new(
                            self.generate_setup_template(&volume_name, config)?,
                        )),
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

                status: None,
            })
            .add_owner_reference(oref)
            .clone())
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

    let task_phase: &TaskPhase = if let Some(status) = &task.status {
        match &status.phase {
            Some(p) => p,
            None => &TaskPhase::Pending,
        }
    } else {
        &TaskPhase::Pending
    };

    if task_phase == &TaskPhase::Failed || task_phase == &TaskPhase::Succeeded {
        return Ok(Action::requeue(Duration::from_secs(50)));
    }

    if task_phase == &TaskPhase::Pending && task.status.is_none() {
        let new_task = Task {
            metadata: task.metadata.clone(),
            spec: TaskSpec::default(),
            status: Some(TaskStatus {
                phase: Some(TaskPhase::Pending),
                reason: None,
            }),
        };

        tasks
            .replace_status(
                &task.name_any(),
                &PostParams::default(),
                serde_json::to_vec(&new_task)?,
            )
            .await?;
        return Ok(Action::requeue(Duration::from_secs(50)));
    }

    let correct_wf = &task.generate_workflow(&ctx.config)?;
    let wf = match workflows
        .patch(
            &correct_wf.name_any(),
            &PatchParams {
                ..PatchParams::apply("taskmanager.teainspace.com")
            },
            &kube::api::Patch::Apply(correct_wf),
        )
        .await
    {
        Ok(wf) => wf,
        Err(e) => {
            println!("error: {:?}", e);
            panic!()
        }
    };

    if let Some(wf_status) = &wf.status {
        let correct_task_phase = match wf_status.phase {
            WorkflowPhase::Running => TaskPhase::Running,
            WorkflowPhase::Pending => TaskPhase::Pending,
            WorkflowPhase::Succeeded => TaskPhase::Succeeded,
            WorkflowPhase::Failed => TaskPhase::Failed,
            WorkflowPhase::Error => TaskPhase::Failed,
        };

        if &correct_task_phase != task_phase {
            let new_task = Task {
                metadata: task.metadata.clone(),
                spec: TaskSpec::default(),
                status: Some(TaskStatus {
                    phase: Some(correct_task_phase),
                    reason: None,
                }),
            };

            let res = tasks
                .patch_status(
                    &task.name_any(),
                    &PatchParams::default(),
                    &kube::api::Patch::Apply(new_task),
                )
                .await;

            match res {
                Ok(_) => println!("Patched status for task: {} ", task.name_any()),
                Err(e) => {
                    println!("Failed to set status: {}", e);
                    return Err(Error::KubeApiError(e));
                }
            };
        }
    }

    Ok(Action::requeue(Duration::from_secs(50)))
}

fn error_policy(task: Arc<Task>, error: &Error, _ctx: Arc<Context>) -> Action {
    println!("error: {}, for task: {}", error, task.name_any());
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

    Controller::new(tasks, ListParams::default())
        .owns(workflows, ListParams::default())
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
        }
    }

    /// Prepare a cluster for tests, under the assumptions that the `just setup_cluster` recipe has been run successfully.
    /// This implies that all required custom resource definitions are installed in the cluster.
    /// This function will generate clients and clear all Task and Workflow objects in the cluster.
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
                println!("Deleted collection of tasks: {:?}", status);
            }
        };

        match workflows.delete_collection(&dp, &lp).await? {
            Either::Left(_) => {
                while !workflows.list(&lp).await?.items.is_empty() {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }

            Either::Right(status) => {
                println!("Deleted collection of tasks: {:?}", status);
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
            .labels(&format!("task={}", task_in_cluster.name_any()))
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
    #[test]
    fn task_can_generate_workflow() {
        let t: Task = serde_json::from_value(json!({
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
                    "name": "secret1",
                    "envkey": "KEY1"
                },
                {
                    "name": "secret2",
                    "envkey": "KEY2"
                }
                ]
            }
        }))
        .unwrap();

        let wf: Workflow = t.generate_workflow(&gen_test_config()).unwrap();

        insta::assert_yaml_snapshot!(&wf);
    }

    #[test]
    fn task_can_override_workflow_image() {
        let t: Task = serde_json::from_value(json!({
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
                    "name": "secret1",
                    "envkey": "KEY1"
                },
                {
                    "name": "secret2",
                    "envkey": "KEY2"
                }
                ]
            }
        }))
        .unwrap();

        let wf: Workflow = t.generate_workflow(&gen_test_config()).unwrap();

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
            .labels(&format!("task={}", task_in_cluster.name_any()))
            .timeout(5);

        let mut correct_labels = BTreeMap::new();
        let mut stream = workflows.watch(&lp, "0").await?.boxed();
        while let Some(e) = stream.next().await {
            if let WatchEvent::Added(wf) = e? {
                correct_labels = wf.metadata.labels.clone().unwrap();
                let mut labels = wf.metadata.labels.clone().unwrap();
                labels.insert("task".to_string(), "test".to_string());
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

                e => println!("event: {:?}", e),
            }
        }

        panic!("workflow was not corrected")
    }
}
