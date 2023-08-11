use ame::custom_resources::{project_source::ProjectSrcCtrlCfg, *};

use controller::{
    data_set::{start_data_set_controller, DataSetControllerCfg},
    project::{start_project_controller, ProjectControllerCfg},
    task::{start_task_controller, TaskControllerCfg},
};
use envconfig::Envconfig;
use kube::Client;
use tracing::info;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<()> {
    let mut task_ctrl_cfg = TaskControllerCfg::init_from_env().unwrap();
    let project_src_ctrl_cfg = ProjectSrcCtrlCfg::init_from_env().unwrap();
    let mut project_ctrl_cfg = ProjectControllerCfg::from_env().unwrap();

    let logger = tracing_subscriber::fmt::layer();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let collector = Registry::default().with(logger).with(env_filter);

    tracing::subscriber::set_global_default(collector).unwrap();

    // Start kubernetes controller
    let client = Client::try_default().await?;

    let data_set_controller = start_data_set_controller(DataSetControllerCfg {
        client: client.clone(),
        namespace: task_ctrl_cfg
            .namespace
            .clone()
            .unwrap_or("ame-system".to_string()),
    })
    .await
    .unwrap();

    if task_ctrl_cfg.namespace.is_none() {
        task_ctrl_cfg.namespace = Some("ame-system".to_string());
    }

    info!("Task controller configuration: {:?}", task_ctrl_cfg);

    let task_controller = start_task_controller(client.clone(), task_ctrl_cfg.clone())
        .await
        .unwrap();
    let projectsrc_controller =
        project_source::start_project_source_controller(project_src_ctrl_cfg).await;

    if project_ctrl_cfg.deployment_image.is_none() {
        project_ctrl_cfg.deployment_image = Some(task_ctrl_cfg.executor_image);
    }

    if project_ctrl_cfg.mlflow_url.is_none() {
        project_ctrl_cfg.mlflow_url =
            Some("http://mlflow.ame-system.svc.cluster.local:5000".to_string());
    }

    let project_controller = start_project_controller(client.clone(), project_ctrl_cfg)
        .await
        .unwrap();

    tokio::select! {
        _ = task_controller=> println!("task controller exited"),
        _ = projectsrc_controller => println!("project source controller exited"),
        _ = project_controller=> println!("project controller exited"),
        _ = data_set_controller => println!("data set controller exited")
    }

    Ok(())
}
