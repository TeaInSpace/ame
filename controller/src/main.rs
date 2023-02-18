use controller::project::ProjectCtrlCfg;
use controller::*;
use controller::{manager::TaskControllerConfig, project_source::ProjectSrcCtrlCfg};
use envconfig::Envconfig;
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
    let task_controller = manager::start_task_controller(task_ctrl_cfg).await;
    let projectsrc_controller =
        project_source::start_project_source_controller(project_src_ctrl_cfg).await;
    let project_controller = project::start_project_controller(project_ctrl_cfg).await;

    tokio::select! {
        _ = task_controller=> println!("task controller exited"),
        _ = projectsrc_controller => println!("project source controller exited"),
        _ = project_controller=> println!("project controller exited"),
    }

    Ok(())
}
