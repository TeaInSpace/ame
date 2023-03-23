use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use controller::secrets::SecretCtrl;
use controller::{
    common::{find_ame_endpoint, private_repo_gh_pat, setup_cluster},
    project_source_ctrl::ProjectSrcCtrl,
};
use controller::{ModelValidationStatus, Project};
use fs_extra::dir::CopyOptions;

use futures_util::StreamExt;

use insta::assert_snapshot;
use k8s_openapi::api::apps::v1::Deployment;
use kube::api::ListParams;
use kube::runtime::{watcher, WatchStreamExt};
use kube::Api;
use kube::Client;
use rstest::*;
use serial_test::serial;
use std::time::Duration;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

static AME_FILE_NAME: &str = "ame.yaml";
static INGRESS_NAMESPACE: &str = "ingress-nginx";
static INGRESS_SERVICE: &str = "ingress-nginx-controller";
static AME_NAMESPACE: &str = "ame-system";
static PRIVATE_GH_REPO_SECRET_KEY: &str = "org-secret";

async fn test_setup() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var(
        "AME_ENDPOINT",
        find_ame_endpoint(INGRESS_NAMESPACE, INGRESS_SERVICE).await?,
    );

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

#[test]
fn ame_file_already_exists() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let test_file = temp.child(AME_FILE_NAME);
    test_file.touch()?;

    let mut cmd = Command::cargo_bin("cli")?;

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("myproject")
        .assert()
        .success();

    // If a file already exists we expect the CLI to inform the user and exist gracefully.
    // Therefore nothing should be written.
    test_file.assert("");

    Ok(())
}

#[test]
fn ame_file_does_not_exist() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let test_file = temp.child(AME_FILE_NAME);
    let mut cmd = Command::cargo_bin("cli")?;
    let project_id = "myproject";

    cmd.current_dir(temp.path())
        .arg("init")
        .arg(project_id)
        .assert()
        .success();

    test_file.assert(format!("projectid: {}\n", &project_id));

    Ok(())
}

#[rstest]
#[case("test_data/test_projects/new_echo", "echo")]
#[case("test_data/test_projects/sklearn_logistic_regression", "training")]
#[ignore]
#[tokio::test]
async fn ame_run_task(
    #[case] project_dir: &str,
    #[case] task_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    setup_cluster("ame-system").await?;
    let temp = prepare_test_project(project_dir)?;
    let mut cmd = Command::cargo_bin("cli")?;
    test_setup().await?;

    let res = cmd
        .current_dir(temp)
        .arg("run")
        .arg(task_id)
        .assert()
        .success();

    let mut settings = insta::Settings::clone_current();
    settings.add_filter("time=\".*\"", "timestamp=\"redacted\"");
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
    let _guard = settings.bind_to_scope();

    insta::assert_snapshot!(&String::from_utf8(res.get_output().stdout.clone())?);

    Ok(())
}

#[tokio::test]
#[serial]
async fn ame_setup_cli() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;

    let temp_path = temp.to_str().unwrap();

    let service_endpoint = find_ame_endpoint(INGRESS_NAMESPACE, INGRESS_SERVICE)
        .await
        .unwrap();

    temp_env::with_vars(
        vec![("AME_ENDPOINT", None), ("XDG_CONFIG_HOME", Some(temp_path))],
        || {
            Command::cargo_bin("cli")
                .unwrap()
                .current_dir(temp_path)
                .arg("setup")
                .arg(service_endpoint.clone())
                .assert()
                .success();
        },
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn fail_bad_server_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;

    let temp_path = temp.to_str().unwrap();

    let service_endpoint = "wrong_endpoint".to_string();

    temp_env::with_vars(
        vec![("AME_ENDPOINT", None), ("XDG_CONFIG_HOME", Some(temp_path))],
        || {
            let output = Command::cargo_bin("cli")
                .unwrap()
                .current_dir(temp_path)
                .arg("setup")
                .arg(service_endpoint.clone())
                .assert();

            assert_snapshot!(&String::from_utf8(output.get_output().stdout.clone()).unwrap());
        },
    );

    Ok(())
}

#[fixture]
async fn kube_client() -> Result<Client, kube::Error> {
    Client::try_default().await
}

#[rstest]
#[case::public_repo(vec!["https://github.com/TeaInSpace/ame-demo.git"], true)]
#[case::none_existent_repo(vec!["https://github.com/TeaInSpace/fake-repo.git"], false)]
#[case::private_repo(vec!["https://github.com/TeaInSpace/ame-test-private.git", "-s", PRIVATE_GH_REPO_SECRET_KEY, "-u", "jmintb"], true)]
#[case::fake_secret(vec!["https://github.com/TeaInSpace/ame-test-private.git", "-s", "secretdoestexist", "-u", "jmintb"], false)]
#[tokio::test]
#[serial]
#[ignore]
async fn can_create_project_source(
    #[case] args: Vec<&str>,
    #[case] should_succeed: bool,
    #[future] kube_client: Result<Client, kube::Error>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter("([⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏].*)", "");
    let _guard = settings.bind_to_scope();

    test_setup().await?;

    let secret_ctrl = SecretCtrl::new(kube_client.await?, AME_NAMESPACE);

    secret_ctrl
        .store_secret_if_empty(PRIVATE_GH_REPO_SECRET_KEY, private_repo_gh_pat()?)
        .await?;

    let mut cmd = Command::cargo_bin("cli")?;

    let output = cmd
        .arg("projectsrc")
        .arg("create")
        .args(args.clone())
        .assert();

    /*
    assert_snapshot!(
        format!(
            "can_create_project_source::case::{:?}::{:?}",
            args, should_succeed
        ),
        &String::from_utf8(output.get_output().stdout.clone())?
    );
    */

    if should_succeed {
        output.success();
    } else {
        output.failure();
    }

    let src_ctrl = ProjectSrcCtrl::try_namespaced(AME_NAMESPACE).await?;
    src_ctrl.delete_project_src_for_repo(args[0]).await?;

    secret_ctrl
        .delete_secret(PRIVATE_GH_REPO_SECRET_KEY)
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn cannot_create_multiple_sources_for_the_same_repo() -> Result<(), Box<dyn std::error::Error>>
{
    test_setup().await?;
    let repo = "https://github.com/TeaInSpace/ame-demo.git";
    let project_src_ctrl = ProjectSrcCtrl::new(kube_client().await?, AME_NAMESPACE);

    let _ = project_src_ctrl.delete_project_src_for_repo(repo).await;

    let mut cmd = Command::cargo_bin("cli")?;

    let _output = cmd
        .arg("projectsrc")
        .arg("create")
        .arg(repo)
        .assert()
        .success();

    cmd = Command::cargo_bin("cli")?;
    let _output = cmd
        .arg("projectsrc")
        .arg("create")
        .arg(repo)
        .assert()
        .failure();

    project_src_ctrl.delete_project_src_for_repo(repo).await?;

    Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn can_use_data_set_train_validate_and_deploy_model() -> Result<(), Box<dyn std::error::Error>>
{
    test_setup().await?;

    // The template repo is required as the  ame-demo requires it to train.
    let template_repo = "https://github.com/TeaInSpace/ame-template-demo.git";
    let data_set_repo = "https://github.com/TeaInSpace/ame-dataset-demo.git";
    let repo = "https://github.com/TeaInSpace/ame-demo.git";
    let model_name = "logreg"; // this name from the ame-demo repo.
    let project_src_ctrl = ProjectSrcCtrl::new(kube_client().await?, AME_NAMESPACE);

    let kube_client = kube_client().await?;
    let deployments: Api<Deployment> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);
    let projects: Api<Project> = Api::namespaced(kube_client.clone(), AME_NAMESPACE);
    // TODO: whyid this no throw an error with AME_FILE_NAME.
    let secret_ctrl = SecretCtrl::try_default(AME_NAMESPACE).await?;
    let s3_secret = "zuf+tfteSlswRu7BJ86wekitnifILbZam1KYY3TG";

    secret_ctrl
        .store_secret("s3secretkey", s3_secret.to_string())
        .await?;

    let _ = project_src_ctrl.delete_project_src_for_repo(repo).await;
    let _ = project_src_ctrl
        .delete_project_src_for_repo(data_set_repo)
        .await;
    let _ = project_src_ctrl
        .delete_project_src_for_repo(template_repo)
        .await;

    let mut cmd = Command::cargo_bin("cli")?;

    let _output = cmd
        .arg("projectsrc")
        .arg("create")
        .arg(template_repo)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("cli")?;

    let _output = cmd
        .arg("projectsrc")
        .arg("create")
        .arg(data_set_repo)
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("cli")?;

    let _output = cmd
        .arg("projectsrc")
        .arg("create")
        .arg(repo)
        .assert()
        .success();

    // Note that the model will start training now if now version is present.
    // We will need to check for this in future tests.

    // It is important that we only have 2 projects as the watcher
    // does not perform any filtering.
    assert_eq!(
        projects
            .list_metadata(&ListParams::default())
            .await?
            .items
            .len(),
        3
    );

    // No deployment for the model should be present.
    assert!(deployments.get(model_name).await.is_err());

    let mut project_watcher = watcher(projects, ListParams::default())
        .applied_objects()
        .boxed();

    while let Some(e) = project_watcher.next().await {
        let Some(mut status) =  e?.status else {
            continue;
        };

        let Some(model_status) = status.get_model_status(model_name) else {
            continue ;
        };

        match model_status.validation {
            Some(ModelValidationStatus::Validated { .. }) => break,
            Some(ModelValidationStatus::FailedValidation { .. }) => {
                return Err("model failed validation".into());
            }
            _ => (),
        };
    }

    let mut deployment_watcher = watcher(
        deployments,
        ListParams::default().fields(&format!("metadata.name=={model_name}")),
    )
    .applied_objects()
    .boxed();

    let timeout = Duration::from_secs(60);
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

    project_src_ctrl.delete_project_src_for_repo(repo).await?;
    project_src_ctrl
        .delete_project_src_for_repo(data_set_repo)
        .await?;
    project_src_ctrl
        .delete_project_src_for_repo(template_repo)
        .await?;

    Ok(())
}

#[ignore]
#[tokio::test]
#[serial]
async fn can_delete_project_src() -> Result<(), Box<dyn std::error::Error>> {
    let repo = "https://github.com/TeaInSpace/ame-demo.git";
    let project_src_ctrl = ProjectSrcCtrl::new(kube_client().await?, AME_NAMESPACE);

    project_src_ctrl
        .create_project_src(&ame::grpc::ProjectSourceCfg::from_git_repo(
            repo.to_string(),
        ))
        .await?;

    let mut cmd = Command::cargo_bin("cli")?;
    cmd.arg("projectsrc")
        .arg("delete")
        .arg(repo)
        .assert()
        .success();

    assert_eq!(project_src_ctrl.list_project_src().await?.len(), 0);

    Ok(())
}

#[ignore]
#[tokio::test]
#[serial]
async fn can_list_project_srcs() -> Result<(), Box<dyn std::error::Error>> {
    let repos = vec![
        "https://github.com/TeaInSpace/ame-demo.git",
        "https://github.com/TeaInSpace/ame-template-demo.git",
    ];
    let project_src_ctrl = ProjectSrcCtrl::new(kube_client().await?, AME_NAMESPACE);

    for repo in &repos {
        project_src_ctrl
            .create_project_src(&ame::grpc::ProjectSourceCfg::from_git_repo(
                repo.to_string(),
            ))
            .await?;
    }

    let mut cmd = Command::cargo_bin("cli")?;
    let output = cmd.arg("projectsrc").arg("list").assert().success();

    assert_snapshot!(&String::from_utf8(output.get_output().stdout.clone())?);

    for repo in &repos {
        project_src_ctrl.delete_project_src_for_repo(repo).await?;
    }

    Ok(())
}

#[ignore]
#[tokio::test]
#[serial]
async fn can_edit_project_src() -> Result<(), Box<dyn std::error::Error>> {
    let repo = "https://github.com/TeaInSpace/ame-demo.git";
    let project_src_ctrl = ProjectSrcCtrl::new(kube_client().await?, AME_NAMESPACE);

    let secret = "somesecret";
    let username = "myuser";

    project_src_ctrl
        .create_project_src(&ame::grpc::ProjectSourceCfg::from_git_repo(
            repo.to_string(),
        ))
        .await?;

    let mut cmd = Command::cargo_bin("cli")?;
    cmd.arg("projectsrc")
        .arg("edit")
        .arg(repo)
        .arg("--secret")
        .arg(secret)
        .arg("--user")
        .arg(username)
        .assert()
        .success();

    assert_eq!(project_src_ctrl.list_project_src().await?.len(), 1);

    let project_src = project_src_ctrl.get_project_src_for_repo(repo).await?;

    assert_eq!(
        project_src
            .spec
            .cfg
            .git
            .as_ref()
            .unwrap()
            .secret
            .as_ref()
            .expect("expect secret to exist"),
        secret
    );
    assert_eq!(
        project_src
            .spec
            .cfg
            .git
            .unwrap()
            .username
            .expect("expect username to exist"),
        username
    );

    project_src_ctrl.delete_project_src_for_repo(repo).await?;

    Ok(())
}
