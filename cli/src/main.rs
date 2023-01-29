use ame_client::ame_service_client::AmeServiceClient;
use clap::{Parser, Subcommand};
use cli::{project::Project, CliConfiguration, Result};
use tonic::Request;

#[derive(Parser)]
#[command()]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        name: String,
    },
    Run {
        name: String,
    },
    Setup {
        endpoint: String,
    },
    Train {
        project: String,
        model: String,
    },
    #[command(subcommand)]
    Create(CreateCommands),
}

#[derive(Subcommand)]
enum CreateCommands {
    Projectsrc { repository: String },
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
        Commands::Train { project, model } => {
            let mut client = AmeServiceClient::connect(config.endpoint).await?;

            client
                .train_model(Request::new(ame_client::TrainRequest {
                    projectid: project.to_string(),
                    model_name: model.to_string(),
                }))
                .await?;
            Ok(())
        }
        Commands::Create(CreateCommands::Projectsrc { repository }) => {
            let mut client = AmeServiceClient::connect(config.endpoint).await?;

            client
                .create_project_src(Request::new(ame_client::ProjectSource {
                    git: Some(ame_client::GitProjectSource {
                        repository: repository.to_string(),
                        sync_interval: Some("10s".to_string()),
                        ..ame_client::GitProjectSource::default()
                    }),
                }))
                .await?;

            Ok(())
        }
    }
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
