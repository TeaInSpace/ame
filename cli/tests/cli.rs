use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use common::{find_ame_endpoint, setup_cluster};
use fs_extra::dir::CopyOptions;
use insta::assert_snapshot;
use rstest::*;
use serial_test::serial;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

static AME_FILE_NAME: &str = "ame.yaml";
static INGRESS_NAMESPACE: &str = "ingress-nginx";
static INGRESS_SERVICE: &str = "ingress-nginx-controller";

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
