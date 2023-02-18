use std::time::Duration;

use ame_client::{
    auth::browser_login,
    client_builder::{build_ame_client, AmeServiceClientCfg},
    GitProjectSource, ProjectSourceCfg, ProjectSourceListParams, ProjectSourceState,
    ProjectSrcIdRequest, ProjectSrcPatchRequest, TaskIdentifier,
};
use clap::{Parser, Subcommand};
use cli::{
    project::Project,
    secrets::{exec_secret_command, SecretCommand},
    CliConfiguration, Result,
};
use colored::Colorize;
use futures_util::StreamExt;
use http::StatusCode;
use spinners::Spinner;
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
    Projectsrc(ProjectSrcCommands),
    #[command(subcommand)]
    Secret(SecretCommand),
}

#[derive(Subcommand)]
enum ProjectSrcCommands {
    Create {
        repository: String,
        #[arg(short, long)]
        secret: Option<String>,
        #[arg(short, long)]
        user: Option<String>,
    },

    Delete {
        repository: Option<String>,
    },

    Edit {
        repository: String,
        #[arg(short, long)]
        secret: Option<String>,
        #[arg(short, long)]
        user: Option<String>,
    },

    List,
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
        Commands::Projectsrc(ProjectSrcCommands::Delete { repository }) => {
            let mut client = build_ame_client(AmeServiceClientCfg {
                disable_tls_cert_check: true,
                endpoint: config.endpoint.parse().unwrap(),
                id_token: config.id_token,
            })
            .await?;

            let repository = repository.as_ref().unwrap();

            let id = client
                .get_project_src_id(Request::new(ProjectSrcIdRequest {
                    repo: repository.to_string(),
                }))
                .await?
                .into_inner();

            client.delete_project_src(Request::new(id)).await?;

            Ok(())
        }

        Commands::Projectsrc(ProjectSrcCommands::List) => {
            let mut client = build_ame_client(AmeServiceClientCfg {
                disable_tls_cert_check: true,
                endpoint: config.endpoint.parse().unwrap(),
                id_token: config.id_token,
            })
            .await?;

            let srcs = client
                .list_project_srcs(Request::new(ProjectSourceListParams {}))
                .await?
                .into_inner();

            println!("{}", "Project Sources:".bright_white().bold());

            for cfg in srcs.cfgs {
                if let ProjectSourceCfg {
                    git:
                        Some(GitProjectSource {
                            repository,
                            username,
                            secret,
                            ..
                        }),
                } = cfg
                {
                    println!(
                        "{} {} {}",
                        repository,
                        username.unwrap_or("".to_string()),
                        secret.unwrap_or("".to_string())
                    );
                }
            }

            Ok(())
        }
        Commands::Projectsrc(ProjectSrcCommands::Edit {
            repository,
            secret,
            user,
        }) => {
            let mut client = build_ame_client(AmeServiceClientCfg {
                disable_tls_cert_check: true,
                endpoint: config.endpoint.parse().unwrap(),
                id_token: config.id_token,
            })
            .await?;

            let id = client
                .get_project_src_id(Request::new(ProjectSrcIdRequest {
                    repo: repository.to_string(),
                }))
                .await?
                .into_inner();

            client
                .update_project_src(Request::new(ProjectSrcPatchRequest {
                    id: Some(id.clone()),
                    cfg: Some(ame_client::ProjectSourceCfg {
                        git: Some(ame_client::GitProjectSource {
                            repository: repository.to_string(),
                            sync_interval: Some("10s".to_string()),
                            secret: secret.clone(),
                            username: user.clone(),
                        }),
                    }),
                }))
                .await?
                .into_inner();

            let mut sp = Spinner::new(
                spinners::Spinners::Dots,
                format!("{} project source", "Updating".bold().cyan()),
            );

            let mut strm = client
                .watch_project_src(Request::new(id))
                .await?
                .into_inner();

            while let Some(entry) = strm.next().await {
                if let Ok(ame_client::ProjectSourceStatus { state, reason, .. }) = entry {
                    match ProjectSourceState::from_i32(state) {
                        Some(ProjectSourceState::Synchronized) => {
                            sp.stop_and_persist(
                                " ",
                                format!("{} project source", "Updated".bold().green()),
                            );
                            break;
                        }
                        Some(ProjectSourceState::Pending) => (),
                        Some(ProjectSourceState::Error) => {
                            sp.stop_and_persist(
                                " ",
                                format!(
                                    "{} to synchronize project source, reason: {}",
                                    "Failed".bold().red(),
                                    reason.unwrap_or("no reason :(".to_string()),
                                ),
                            );
                            std::process::exit(1);
                        }
                        _ => {
                            sp.stop_and_persist(
                                " ",
                                format!("{} to update project source", "Failed".bold().red()),
                            );
                            std::process::exit(1);
                        }
                    }
                }
            }

            sp = Spinner::new(
                spinners::Spinners::Dots,
                format!("{} projects from source", "Creating".bold().cyan()),
            );

            tokio::time::sleep(Duration::from_secs(5)).await;

            sp.stop_and_persist(
                " ",
                format!("{} projects from source", "Created".bold().green()),
            );

            Ok(())
        }
        Commands::Projectsrc(ProjectSrcCommands::Create {
            repository,
            secret,
            user,
        }) => {
            let mut client = build_ame_client(AmeServiceClientCfg {
                disable_tls_cert_check: true,
                endpoint: config.endpoint.parse().unwrap(),
                id_token: config.id_token,
            })
            .await?;

            let id = client
                .create_project_src(Request::new(ame_client::ProjectSourceCfg {
                    git: Some(ame_client::GitProjectSource {
                        repository: repository.to_string(),
                        sync_interval: Some("10s".to_string()),
                        secret: secret.clone(),
                        username: user.clone(),
                    }),
                }))
                .await?
                .into_inner();

            let mut sp = Spinner::new(
                spinners::Spinners::Dots,
                format!("{} project source", "Creating".bold().cyan()),
            );

            let mut strm = client
                .watch_project_src(Request::new(id))
                .await?
                .into_inner();

            tokio::time::sleep(Duration::from_secs(2)).await;

            loop {
                let entry = strm.next().await;
                let Some(entry) = entry else {
                            sp.stop_and_persist(
                                " ",
                                format!("{} to create project source", "Failed".bold().red()),
                            );
                            std::process::exit(1);
                };
                if let Ok(ame_client::ProjectSourceStatus { state, reason, .. }) = entry {
                    match ProjectSourceState::from_i32(state) {
                        Some(ProjectSourceState::Synchronized) => {
                            sp.stop_and_persist(
                                " ",
                                format!("{} project source", "Created".bold().green()),
                            );
                            break;
                        }
                        Some(ProjectSourceState::Pending) => (),
                        Some(ProjectSourceState::Error) => {
                            sp.stop_and_persist(
                                " ",
                                format!(
                                    "{} to synchronize project source, reason: {}",
                                    "Failed".bold().red(),
                                    reason.unwrap_or("no reason :(".to_string()),
                                ),
                            );
                            std::process::exit(1);
                        }
                        _ => {
                            sp.stop_and_persist(
                                " ",
                                format!("{} to create project source", "Failed".bold().red()),
                            );
                            std::process::exit(1);
                        }
                    }
                }
            }

            sp.stop_and_persist(" ", format!("{} project source", "Created".bold().green()));

            Ok(())
        }
        Commands::Secret(cmd) => exec_secret_command(config, cmd).await,
    }
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
