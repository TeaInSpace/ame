use controller::manager::TaskControllerConfig;
pub use controller::*;
use envconfig::Envconfig;

#[tokio::main]
async fn main() -> Result<()> {
    let config = TaskControllerConfig::init_from_env().unwrap();

    // Start kubernetes controller
    let controller = manager::start_task_controller(config).await;
    tokio::select! {
        _ = controller => println!("controller exited"),
    }

    Ok(())
}
