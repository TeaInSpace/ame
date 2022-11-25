use ame_cli::{project::Project, CliConfiguration, Result};
use ame_client::ame_service_client::AmeServiceClient;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init { name: String },
    Run { name: String },
    Setup { endpoint: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = CliConfiguration::gather()?;

    match &cli.command {
        // TODO: if an error is returned here the output will be confusing to the user.
        Commands::Init { name } => Project::init(name),
        Commands::Run { name: name_arg } => {
            let task_template_name = name_arg.as_ref();
            let project = Project::init_from_working_dir()?;
            let client = AmeServiceClient::connect(config.endpoint).await?;

            project.run_task(client, task_template_name).await?;

            Ok(())
        }
        Commands::Setup { endpoint } => {
            CliConfiguration {
                endpoint: endpoint.to_string(),
            }
            .save()?;
            println!("configuration saved!");

            AmeServiceClient::connect(CliConfiguration::gather()?.endpoint).await?;

            Ok(())
        }
    }
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
