use ame::custom_resources::project::ProjectCtrlCfg;
use ame::custom_resources::*;
use ame::custom_resources::{
    project_source::ProjectSrcCtrlCfg, task::start_task_controller, task::TaskControllerConfig,
};
use controller::data_sets::{start_data_set_controller, DataSetControllerCfg};
use envconfig::Envconfig;
use kube::Client;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

#[tokio::main]
async fn main() -> Result<()> {
    let task_ctrl_cfg = TaskControllerConfig::init_from_env().unwrap();
    let project_src_ctrl_cfg = ProjectSrcCtrlCfg::init_from_env().unwrap();
    let project_ctrl_cfg = ProjectCtrlCfg::from_env().unwrap();

    let logger = tracing_subscriber::fmt::layer();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let collector = Registry::default().with(logger).with(env_filter);

    tracing::subscriber::set_global_default(collector).unwrap();

    // Start kubernetes controller
    let client = Client::try_default().await?;
    let data_set_controller = start_data_set_controller(DataSetControllerCfg {
        client,
        namespace: task_ctrl_cfg.namespace.clone(),
    })
    .await
    .unwrap();
    let task_controller = start_task_controller(task_ctrl_cfg).await;
    let projectsrc_controller =
        project_source::start_project_source_controller(project_src_ctrl_cfg).await;
    let project_controller = project::start_project_controller(project_ctrl_cfg).await;

    tokio::select! {
        _ = task_controller=> println!("task controller exited"),
        _ = projectsrc_controller => println!("project source controller exited"),
        _ = project_controller=> println!("project controller exited"),
        _ = data_set_controller => println!("data set controller exited")
    }

    Ok(())
}
