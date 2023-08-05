use ame::custom_resources::{
    common::{find_ame_endpoint, private_repo_gh_pat, setup_cluster},
    project_source_ctrl::ProjectSrcCtrl,
    secrets::SecretCtrl,
};
use assert_cmd::prelude::*;
use assert_fs::prelude::*;

use fs_extra::dir::CopyOptions;
use insta::assert_snapshot;

use anyhow::anyhow;
use kube::Client;
use rstest::*;
use serial_test::serial;

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

#[test]
fn ame_file_already_exists() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let test_file = temp.child(AME_FILE_NAME);
    test_file.touch()?;

    let mut cmd = Command::cargo_bin("ame")?;

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("myproject")
        .assert()
        .success();

    // If a file already exists we expect the CLI to inform the user and exit gracefully.
    // Therefore nothing should be written.
    test_file.assert("");

    Ok(())
}

#[test]
fn ame_file_does_not_exist() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let test_file = temp.child(AME_FILE_NAME);
    let mut cmd = Command::cargo_bin("ame")?;
    let project_id = "myproject";

    cmd.current_dir(temp.path())
        .arg("init")
        .arg(project_id)
        .assert()
        .success();

    test_file.assert(format!("name: {}\n", &project_id));

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
            Command::cargo_bin("ame")
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
            let output = Command::cargo_bin("ame")
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

fn prepare_test_project(path_from_root: &str) -> anyhow::Result<PathBuf> {
    let mut test_project_path = PathBuf::from(format!("test_data/test_projects/{path_from_root}"));

    for i in 0..10 {
        if test_project_path.is_dir() && i < 9 {
            break;
        }

        if i < 9 {
            test_project_path = Path::new("..").join(test_project_path);
            continue;
        }

        return Err(anyhow!(
            "failed to find test project directory: {}",
            test_project_path.display()
        ));
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

// TODO; test failure cases and messages
#[tokio::test]
#[cfg(ignored)]
async fn can_remove_task() -> anyhow::Result<()> {
    let _ = setup_cluster(AME_NAMESPACE).await;
    let temp = prepare_test_project("executors/poetry")?;
    let mut cmd = Command::cargo_bin("ame")?;

    cmd.current_dir(temp.clone());

    cmd.arg("task")
        .arg("run")
        .arg("training")
        .assert()
        .success();

    let tasks: Api<Task> = Api::namespaced(kube_client().await?, AME_NAMESPACE);

    let task_name = tasks.list(&ListParams::default()).await?.items[0].name_any();
    let mut cmd = Command::cargo_bin("ame")?;

    cmd.current_dir(temp);

    cmd.args(["task", "remove", "--approve", &task_name])
        .assert()
        .success();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let task_list = tasks.list(&ListParams::default()).await?.items;
    assert!(
        task_list.is_empty(),
        "found tasks {:?}",
        task_list
            .iter()
            .map(|t| t.name_any())
            .collect::<Vec<String>>()
    );

    Ok(())
}

#[tokio::test]
#[cfg(ignored)]
async fn snap_shot_task_view() -> anyhow::Result<()> {
    let _ = setup_cluster(AME_NAMESPACE).await;
    let temp = prepare_test_project("executors/poetry")?;
    let mut cmd = Command::cargo_bin("ame")?;

    cmd.current_dir(temp.clone());

    cmd.arg("task")
        .arg("run")
        .arg("training")
        .assert()
        .success();

    let tasks: Api<Task> = Api::namespaced(kube_client().await?, AME_NAMESPACE);
    let task_name = tasks.list(&ListParams::default()).await?.items[0].name_any();
    let mut cmd = Command::cargo_bin("ame")?;

    cmd.current_dir(temp);

    let res = cmd.args(["task", "view", &task_name]).assert();

    insta::assert_snapshot!(String::from_utf8(res.get_output().stdout.clone())?);

    Ok(())
}

#[rstest]
#[case::invalid_project_file("invalid_project", &["task", "run"])]
#[case::missing_project_file("missing_project", &["task", "run"])]
// #[case::can_reach_server("invalid_project", &["task", "logs"])]
fn snap_shot_test_cli_error_messages(
    #[case] project_dir: &str,
    #[case] args: &[&str],
) -> anyhow::Result<()> {
    let temp = prepare_test_project(project_dir)?;
    let mut cmd = Command::cargo_bin("ame")?;

    let res = cmd.current_dir(temp.clone()).args(args).assert().failure();

    insta::assert_snapshot!(
        format!("{}{}", project_dir.to_string(), args.join("").to_string()),
        &String::from_utf8(res.get_output().stderr.clone())?
    );

    Ok(())
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

    let _ = setup_cluster(AME_NAMESPACE).await;

    test_setup().await?;

    let secret_ctrl = SecretCtrl::new(kube_client.await?, AME_NAMESPACE);

    secret_ctrl
        .store_secret_if_empty(PRIVATE_GH_REPO_SECRET_KEY, private_repo_gh_pat()?)
        .await?;

    let mut cmd = Command::cargo_bin("ame")?;

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
    let _ = src_ctrl.delete_project_src_for_repo(args[0]).await;

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

    let mut cmd = Command::cargo_bin("ame")?;

    let _output = cmd
        .arg("projectsrc")
        .arg("create")
        .arg(repo)
        .assert()
        .success();

    cmd = Command::cargo_bin("ame")?;
    let _output = cmd
        .arg("projectsrc")
        .arg("create")
        .arg(repo)
        .assert()
        .failure();

    project_src_ctrl.delete_project_src_for_repo(repo).await?;

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

    let mut cmd = Command::cargo_bin("ame")?;
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

    let mut cmd = Command::cargo_bin("ame")?;
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

    let mut cmd = Command::cargo_bin("ame")?;
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
