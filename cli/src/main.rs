use ame_client::{
    auth::browser_login,
    client_builder::{build_ame_client, AmeServiceClientCfg},
    TaskIdentifier,
};
use clap::{Parser, Subcommand};
use cli::{project::Project, CliConfiguration, Result};
use http::StatusCode;
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
    Login,
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
    let mut config = CliConfiguration::gather()?;

    match &cli.command {
        // TODO: if an error is returned here the output will be confusing to the user.
        Commands::Init { name } => Project::init(name),
        Commands::Run { name: name_arg } => {
            let task_template_name = name_arg.as_ref();
            let project = Project::init_from_working_dir()?;
            let mut client = build_ame_client(AmeServiceClientCfg {
                disable_tls_cert_check: true,
                endpoint: config.endpoint.parse().unwrap(),
                id_token: config.id_token,
            })
            .await?;

            project.run_task(&mut client, task_template_name).await?;

            Ok(())
        }
        Commands::Setup { endpoint } => {
            let cli_cfg = CliConfiguration::init_with_endpoint(endpoint.to_string());
            let mut client = build_ame_client(cli_cfg.clone().try_into()?).await?;

            println!("testing connection");

            let res = client
                .get_task(Request::new(TaskIdentifier {
                    name: "testssfsf".to_string(),
                }))
                .await;

            if let Err(res) = res {
                // TODO: Extract HTTP code properly
                if res.to_string().contains("401") {
                    println!("It looks like your AME instance requires authentication, please run 'ame login'")
                } else if res.clone().to_http().status() != StatusCode::NOT_FOUND {
                    println!(
                        "Could not reach an AME endpoint at: {}, {:?}",
                        cli_cfg.endpoint, res
                    );
                }
                cli_cfg.save()?;
                println!("configuration saved!");
            }

            Ok(())
        }
        Commands::Login => {
            let provider_url = format!(
                "{}/realms/ame",
                config.endpoint.replace("://", "://keycloak.")
            );

            tracing::debug!(
                "initiating login, with client ID: {} and issuer URL: {:?}",
                "ame-cli",
                provider_url
            );

            let (id_token, access_token, refresh_token) =
                browser_login(provider_url, "ame-cli".to_string()).await?;

            config.set_auth_details(id_token, access_token, refresh_token);
            config.save()?;

            println!("success!");

            Ok(())
        }
        Commands::Train { project, model } => {
            let mut client = build_ame_client(AmeServiceClientCfg {
                disable_tls_cert_check: true,
                endpoint: config.endpoint.parse().unwrap(),
                id_token: config.id_token,
            })
            .await?;

            client
                .train_model(Request::new(ame_client::TrainRequest {
                    projectid: project.to_string(),
                    model_name: model.to_string(),
                }))
                .await?;
            Ok(())
        }
        Commands::Create(CreateCommands::Projectsrc { repository }) => {
            let mut client = build_ame_client(AmeServiceClientCfg {
                disable_tls_cert_check: true,
                endpoint: config.endpoint.parse().unwrap(),
                id_token: config.id_token,
            })
            .await?;

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
