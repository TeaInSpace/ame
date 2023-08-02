use ame::custom_resources::{
    data_set::{DataSet, DataSetPhase, DataSetStatus},
    new_task::Task,
};

use ame::{
    custom_resources::{
        common::setup_cluster, project::Project, project_source_ctrl::ProjectSrcCtrl,
        secrets::SecretCtrl,
    },
    grpc::{task_status::Phase, ProjectCfg, TaskCfg, TaskStatus},
};
use assert_cmd::prelude::*;
use fs_extra::dir::CopyOptions;

use futures_util::StreamExt;

use k8s_openapi::api::{apps::v1::Deployment, core::v1::Service, networking::v1::Ingress};
use kube::{
    api::{DeleteParams, ListParams, PatchParams},
    runtime::{watcher, WatchStreamExt},
    Api, Client,
};
use rstest::*;
use serial_test::serial;
use std::collections::HashMap;

use time::Instant;

use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tracing::{debug, instrument};
use tracing_subscriber::EnvFilter;

static INGRESS_NAMESPACE: &str = "ingress-nginx";
static INGRESS_SERVICE: &str = "ingress-nginx-controller";
static AME_NAMESPACE: &str = "ame-system";

async fn test_setup() -> Result<(), Box<dyn std::error::Error>> {
    // std::env::set_var(
    //     "AME_ENDPOINT",
    //     find_ame_endpoint(INGRESS_NAMESPACE, INGRESS_SERVICE).await?,
    // );
    std::env::set_var("AME_ENDPOINT", "http://localhost:3342");

    Ok(())
}

fn prepare_test_project(path_from_root: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut test_project_path = PathBuf::from(path_from_root);

    for i in 0..10 {
        if test_project_path.is_dir() && i < 9 {
            break;
        }

        if i < 9 {
            test_project_path = Path::new("..").join(test_project_path);
            continue;
        }

        return Err(format!(
            "failed to find test project directory: {}",
            test_project_path.display()
        ))?;
    }

    let temp_dir = assert_fs::TempDir::new()?.into_persistent();

    fs_extra::copy_items(
        &[test_project_path],
        temp_dir.path(),
        &CopyOptions::default(),
    )?;

    Ok(temp_dir
        .path()
        .join(Path::new(path_from_root).file_name().unwrap()))
}

// TODO add concurrent tests
// TODO Test failure handling

#[rstest]
#[trace]
#[case("test_data/test_projects/executors/poetry", "training")]
#[case("test_data/test_projects/executors/pipenv", "training")]
#[case("test_data/test_projects/executors/pip", "training")]
#[case("test_data/test_projects/executors/mlflow", "training")]
#[case(
    "test_data/test_projects/executors/mlflow",
    "training-with-local-template"
)]
#[case(
    "test_data/test_projects/executors/mlflow",
    "training-with-remote-template"
)]
#[case("test_data/test_projects/executors/custom", "training")]
#[case("test_data/test_projects/env", "testenv")]
#[case("test_data/test_projects/env", "testsecret")]
#[ignore]
#[tokio::test]
async fn ame_run_task(
    #[case] project_dir: &str,
    #[case] task_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    init_test_tracing();
    let _ = setup_cluster("ame-system").await; // TODO: async tests are affecting each other here.
    debug!("running task {task_id} in {project_dir}");
    let projects: Api<Project> = Api::namespaced(kube_client().await?, AME_NAMESPACE);
    let project: Project = serde_json::from_value(serde_json::json!(
        {
            "metadata": {
            "name": "shared-templates-project"
                },
            "spec": {
                "name": "shared-templates",
                "templates": [{

                "name": "mlflow-template",
                "executor": {
                    "mlflow": {}
                }}],

                "deletionApproved": false
            }
        }
    ))
    .unwrap();

    projects
        .patch(
            "shared-templates-project",
            &PatchParams::apply("AME-TEST").force(),
            &kube::api::Patch::Apply(project),
        )
        .await
        .unwrap();

    let temp = prepare_test_project(project_dir)?;
    let mut cmd = Command::cargo_bin("ame")?;
    test_setup().await?;

    let res = cmd
        .current_dir(temp.clone())
        .arg("task")
        .arg("run")
        .arg("--logs")
        .arg(task_id)
        .assert()
        .success();

    let mut settings = insta::Settings::clone_current();
    settings.add_filter("time=\".*\".*\n", "");
    settings.add_filter(
        "created virtual environment .*",
        "created virtual environment \"redacted\"",
    );
    settings.add_filter("creator .*", "creator \"redacted\"");
    settings.add_filter(
        "\\d\\d\\d\\d/\\d\\d/\\d\\d \\d\\d:\\d\\d:\\d\\d",
        "\"redacted timestamp\"",
    );
    settings.add_filter("run .*", "\"redacted run ID\"");
    settings.add_filter("ID '.*'", "\"redacted run ID\"");
    settings.add_filter("tmp/tmp.*\\s", "\"redacted temporary directory\"");
    settings.add_filter("mlflow-.*/", "\"redacted MLflow env ID\"");
    settings.add_filter(".*: UserWarning: .*\\n", "");
    settings.add_filter("warnings\\.warn.*\\n", "");
    settings.add_filter("  \"redacted timestamp", "\"redacted timestamp");
    settings.add_filter("  Score:", "Score:");
    settings.add_filter("added seed packages.*", "redacted");
    settings.add_filter("Registered model '.*' already exists.*", "redacted");
    settings.add_filter("Created version '.'.*\\n", "");
    settings.add_filter(", version .", ", version %");
    settings.add_filter("Successfully registered model .*", "redacted");
    // TODO: why does \\. not work here?
    settings.add_filter("\\d+\\.\\d+ MB", "redacted");
    settings.add_filter("\\d+\\.\\d+ KB", "redacted");
    settings.add_filter("tasks/.+/projectfiles", "tasks/redacted/projectfiles");
    let _guard = settings.bind_to_scope();

    // TODO: how can we incorporate snapshots in a non brittle way?
    // insta::assert_snapshot!(&String::from_utf8(res.get_output().stdout.clone())?);

    let project = ProjectCfg::try_from_dir(temp.to_str().unwrap()).unwrap();

    debug!(
        "stdout: {}",
        String::from_utf8(res.get_output().stdout.clone()).unwrap()
    );

    // TODO: should be use the ame API server to monitor the status here?
    let tasks: Api<Task> = Api::namespaced(kube_client().await?.clone(), AME_NAMESPACE);
    let task_list = tasks.list(&ListParams::default()).await?;
    let task_obs: Vec<Task> = task_list
        .items
        .into_iter()
        .filter(|t| {
            t.spec.cfg.name.as_ref().unwrap() == task_id
                && t.metadata
                    .owner_references
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|oref| oref.name.contains(&project.name))
        })
        .collect();

    assert_eq!(
        task_obs.len(),
        1,
        "expected to find a single matching task, instead found {}",
        task_obs.len()
    );

    // TODO: we need to identify each task uniqely here
    assert!(
        task_obs[0]
            .status
            .as_ref()
            .unwrap()
            .phase
            .as_ref()
            .unwrap()
            .success(),
        "task {} was not successful",
        { task_id }
    );

    Ok(())
}
#[rstest]
#[trace]
#[case("test_data/test_projects/executors/poetry", "crontraining")]
#[ignore]
#[tokio::test]
#[serial]
async fn ame_trigger_tasks(
    #[case] project_dir: &str,
    #[case] task_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    init_test_tracing();
    let _ = setup_cluster("ame-system").await; // TODO: async tests are affecting each other here.
    let _projects: Api<Project> = Api::namespaced(kube_client().await?, AME_NAMESPACE);
    let tasks: Api<Task> = Api::namespaced(kube_client().await?, AME_NAMESPACE);

    let temp = prepare_test_project(project_dir)?;
    let mut cmd = Command::cargo_bin("ame")?;
    test_setup().await?;

    let mut project = ProjectCfg::try_from_dir(temp.to_str().unwrap()).unwrap();

    let now = time::OffsetDateTime::now_utc();

    let target = now.checked_add(time::Duration::seconds(270)).unwrap();

    let mut project_file_path = temp.clone();
    project_file_path.push("ame.yaml");

    project.tasks[0].triggers = Some(ame::grpc::TriggerCfg {
        schedule: Some(format!("{} {} * * *", target.minute(), target.hour())),
    });

    let f = std::fs::File::create(project_file_path)?;
    serde_yaml::to_writer(f, &project).unwrap();

    let res = cmd
        .current_dir(temp.clone())
        .arg("project")
        .arg("push")
        .arg("--triggers")
        .assert()
        .success();

    let start_t = Instant::now();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if start_t.elapsed() > time::Duration::seconds(360) {
            return Err("task took too long to complete".to_string().into());
        }
        let task_list = tasks.list(&ListParams::default()).await?;
        let task_obs: Vec<Task> = task_list
            .items
            .into_iter()
            .filter(|t| {
                t.spec.cfg.name.as_ref().unwrap() == task_id
                    && t.metadata
                        .owner_references
                        .as_ref()
                        .unwrap()
                        .iter()
                        .any(|oref| oref.name.contains(&project.name))
            })
            .collect();

        assert!(
            task_obs.len() < 2,
            "expected to find a single or no matching task, instead found {}",
            task_obs.len(),
        );

        let Some(task) = task_obs.get(0) else {
            continue;
        };

        let _now = time::OffsetDateTime::now_utc();

        if start_t.elapsed() < time::Duration::seconds(120) {
            return Err(format!(
                "we do not expect to see the schedule task a head of time {}",
                start_t.elapsed()
            )
            .to_string()
            .into());
        }

        if task
            .status
            .as_ref()
            .unwrap()
            .phase
            .as_ref()
            .unwrap()
            .success()
        {
            break;
        }
    }

    debug!(
        "stdout: {}",
        String::from_utf8(res.get_output().stdout.clone()).unwrap()
    );

    Ok(())
}

// TODO get rid of this, we don't need a fixture for something so simple
#[fixture]
async fn kube_client() -> Result<Client, kube::Error> {
    Client::try_default().await
}

pub async fn delete_registered_model(name: &str, mlflow_url: &str) -> Result<(), reqwest::Error> {
    let mut map = HashMap::new();
    map.insert("name", &name);

    let res = reqwest::Client::new()
        .delete(format!(
            "{mlflow_url}/api/2.0/mlflow/registered-models/delete"
        ))
        .json(&map);

    res.send().await?.error_for_status()?;

    Ok(())
}

fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
}

#[tokio::test]
#[serial]
#[ignore]
#[instrument]
async fn can_use_data_set_train_validate_and_deploy_model() -> Result<(), Box<dyn std::error::Error>>
{
    debug!("can use data, preparing cluster");
    init_test_tracing();
    let _ = setup_cluster(AME_NAMESPACE).await;
    test_setup().await?;

    debug!("Cluster is ready!");

    // The template repo is required as the ame-demo requires it to train.
    let template_repo = "https://github.com/TeaInSpace/ame-template-demo.git";
    let data_set_repo = "https://github.com/TeaInSpace/ame-dataset-demo.git";
    let project_repo = "https://github.com/TeaInSpace/ame-demo.git";
    let model_name = "logreg"; // this name is from the ame-demo repo.
    let project_src_ctrl = ProjectSrcCtrl::new(kube_client().await?, AME_NAMESPACE);

    let kube_client = kube_client().await?;
    let deployments: Api<Deployment> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);
    let datasets: Api<DataSet> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);
    let projects: Api<Project> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);
    let ingress: Api<Ingress> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);
    let services: Api<Service> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);
    let tasks: Api<Task> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);

    // TODO: whyid this not throw an error with AME_FILE_NAME.
    let secret_ctrl = SecretCtrl::try_default(AME_NAMESPACE).await?;
    let s3_secret = "zuf+tfteSlswRu7BJ86wekitnifILbZam1KYY3TG";

    secret_ctrl
        .store_secret_if_empty("s3secretkey", s3_secret.to_string())
        .await?;

    let res = delete_registered_model("logreg", "http://localhost:5000").await;

    // If a model was not present mlflow should return a 404. In this case we can
    // proceed as that does not indicate a problem. Any other error code indicates
    // that something went wrong and the test therefor aborts.
    if let Err(e) = res {
        if let Some(status) = e.status() {
            if status != 404 {
                return Err(format!("received unexpected error from mlflow server {e}").into());
            }
        }
    }

    // TODO: this is very brittle, should be replaced by proper resource ownership.
    let _ = deployments.delete("logreg", &DeleteParams::default()).await;
    let _ = ingress.delete("logreg", &DeleteParams::default()).await;
    let _ = services.delete("logreg", &DeleteParams::default()).await;

    // TODO: test when controller is blocked from reconciling due to a deletion request.

    // Create each project src using the CLI.
    // This should be the only interaction necessary to deploy a model.
    debug!("create project sources with the CLI");
    for repo in [template_repo, data_set_repo, project_repo] {
        let mut cmd = Command::cargo_bin("ame")?;
        let _output = cmd
            .arg("projectsrc")
            .arg("create")
            .arg(repo)
            .assert()
            .success();
    }

    // It is important that we only have 3 projects as the watcher
    // does not perform any filtering. If other projects are present
    // something is interfering with the test.
    assert_eq!(
        projects
            .list_metadata(&ListParams::default())
            .await?
            .items
            .len(),
        3
    );

    // No deployment for the model should be present.
    // TODO: this should be replaced by ownershop somehow.
    assert!(deployments.get(model_name).await.is_err());

    let timeout = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();

    let mut data_set_watcher = watcher(datasets, ListParams::default())
        .applied_objects()
        .boxed();

    debug!("waiting for dataset to be ready");
    // Before the model can be trained we expect a data set to be prepared.
    while let Some(data_set) = data_set_watcher.next().await {
        if let DataSet {
            status:
                Some(DataSetStatus {
                    phase: Some(DataSetPhase::Ready { .. }),
                }),
            ..
        } = data_set?
        {
            break;
        }

        if start.elapsed() > timeout {
            return Err("failed to prepare dataset within timeout, with event"
                .to_string()
                .into());
        }
    }

    let mut task_watcher = watcher(tasks, ListParams::default())
        .applied_objects()
        .boxed();

    // Before the model can be deployed we expect it to be trained and then validated.
    for target_name in ["training", "mlflow_validation"] {
        let start = std::time::Instant::now();
        debug!("waiting for task {target_name}");
        while let Some(e) = task_watcher.next().await {
            let task = e?;

            let TaskCfg {
                name: Some(task_name),
                ..
            } = task.spec.cfg
            else {
                return Err(format!("{target_name} task was missing a name in the cfg").into());
            };

            if task_name != target_name {
                continue;
            }

            if let Some(
                TaskStatus {
                    phase: Some(Phase::Succeeded(_)),
                },
                ..,
            ) = task.status
            {
                break;
            }

            if start.elapsed() > timeout {
                return Err(format!("failed to start task {} within timeout", task_name).into());
            }
        }
    }

    let mut deployment_watcher = watcher(
        deployments,
        ListParams::default().fields(&format!("metadata.name=={model_name}")),
    )
    .applied_objects()
    .boxed();

    debug!("waiting for deployment");
    let start = std::time::Instant::now();
    while let Some(e) = deployment_watcher.next().await {
        let Some(status) = e?.status else {
            return Err("missing deployment status".into());
        };

        if let Some(1) = status.ready_replicas {
            break;
        }

        if start.elapsed() > timeout {
            return Err("failed to deploy model within timeout".into());
        }
    }

    project_src_ctrl
        .delete_project_src_for_repo(project_repo)
        .await?;
    project_src_ctrl
        .delete_project_src_for_repo(data_set_repo)
        .await?;
    project_src_ctrl
        .delete_project_src_for_repo(template_repo)
        .await?;

    Ok(())
}
