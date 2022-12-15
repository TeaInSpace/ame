use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use common::{find_service_endpoint, setup_cluster};
use fs_extra::dir::CopyOptions;
use serial_test::serial;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

static AME_FILE_NAME: &str = "ame.yaml";
static TARGET_NAMESPACE: &str = "ame-system";
static AME_SERVICE_NAME: &str = "ame-server-service";

async fn test_setup() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var(
        "AME_ENDPOINT",
        find_service_endpoint(TARGET_NAMESPACE, AME_SERVICE_NAME).await?,
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

#[tokio::test]
async fn ame_run_task() -> Result<(), Box<dyn std::error::Error>> {
    setup_cluster("ame-system").await?;
    let temp = prepare_test_project("test_data/test_projects/new_echo")?;
    println!("test project {}", temp.display());
    let mut cmd = Command::cargo_bin("cli")?;
    let task_id = "echo";
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
    let _guard = settings.bind_to_scope();
    insta::assert_snapshot!(&String::from_utf8(res.get_output().stdout.clone())?);

    Ok(())
}

#[tokio::test]
#[serial]
async fn ame_setup_cli() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;

    let temp_path = temp.to_str().unwrap();

    let service_endpoint = find_service_endpoint(TARGET_NAMESPACE, AME_SERVICE_NAME)
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
            Command::cargo_bin("cli")
                .unwrap()
                .current_dir(temp_path)
                .arg("setup")
                .arg(service_endpoint.clone())
                .assert()
                .failure();
        },
    );

    Ok(())
}
