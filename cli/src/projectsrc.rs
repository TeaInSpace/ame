use std::time::Duration;

use ame::grpc::GitProjectSource;
use ame::grpc::ProjectSourceCfg;
use ame::grpc::ProjectSourceListParams;
use ame::grpc::ProjectSourceState;
use ame::grpc::ProjectSourceStatus;
use ame::grpc::ProjectSrcIdRequest;
use ame::grpc::ProjectSrcPatchRequest;
use clap::Subcommand;
use colored::Colorize;
use futures_util::StreamExt;
use spinners::Spinner;
use tonic::Request;

use crate::CliConfiguration;
use crate::Result;

///  handles all operations on Project Sources.
///
///  uses project sources to track projects in a Git repositories.
#[derive(Subcommand)]
pub enum ProjectSrcCommands {
    /// Create a new project source pointing to a Git repository.
    ///
    /// For private repositories provide a secret name and associated Git user name.
    ///  will look for the secret in the builtin secret store and any external integrated
    /// stores such as  Vault.
    ///
    /// Use `ame secret list` to view the available secrets.
    Create {
        /// A Git repository.
        repository: String,

        /// The name of the secret.
        #[arg(short, long)]
        secret: Option<String>,

        /// A Git user name that will work with the secret.
        #[arg(short, long)]
        user: Option<String>,
    },

    /// Delete a project source.
    Delete {
        /// Git repository used to identify the Project Source.
        repository: String,
    },

    /// Edit a Project Source pointing to a Git repository
    ///
    /// Any supplied arguments other then the repository will overwrite existing values.
    ///
    /// Example:   edit my..git --secretrepostiory  
    ///
    /// This update the secret used for my..git.
    Edit {
        /// Git repository
        repository: String,

        /// The name of the secret.
        #[arg(short, long)]
        secret: Option<String>,

        /// A Git user name that will work with the secret.
        #[arg(short, long)]
        user: Option<String>,
    },

    /// List all Project Sources
    List,
}

impl ProjectSrcCommands {
    pub async fn run(&self, cfg: &CliConfiguration) -> Result<()> {
        let mut client = cfg.ame_client().await?;

        match self {
            ProjectSrcCommands::Delete { repository } => {
                let id = client
                    .get_project_src_id(Request::new(ProjectSrcIdRequest {
                        repo: repository.to_string(),
                    }))
                    .await?
                    .into_inner();

                client.delete_project_src(Request::new(id)).await?;
            }

            ProjectSrcCommands::List => {
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
            }

            ProjectSrcCommands::Create {
                repository,
                secret,
                user,
            } => {
                let id = client
                    .create_project_src(Request::new(ProjectSourceCfg {
                        git: Some(GitProjectSource {
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

                // wait for initial status.
                let mut retries = 100;
                while (client
                    .get_project_src_status(Request::new(id.clone()))
                    .await)
                    .is_err()
                {
                    if retries <= 0 {
                        todo!("get actual stream working");
                    }

                    retries -= 1;

                    tokio::time::sleep(Duration::from_millis(100)).await;
                }

                loop {
                    let status = client
                        .get_project_src_status(Request::new(id.clone()))
                        .await?
                        .into_inner();

                    match ProjectSourceState::from_i32(status.state) {
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
                                    status.reason.unwrap_or("no reason :(".to_string()),
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

                    tokio::time::sleep(Duration::from_millis(100)).await;
                }

                // TODO: figure out why this panics
                //sp.stop_and_persist(" ", format!("{} project source", "Created".bold().green()));
            }

            ProjectSrcCommands::Edit {
                repository,
                secret,
                user,
            } => {
                let id = client
                    .get_project_src_id(Request::new(ProjectSrcIdRequest {
                        repo: repository.to_string(),
                    }))
                    .await?
                    .into_inner();

                client
                    .update_project_src(Request::new(ProjectSrcPatchRequest {
                        id: Some(id.clone()),
                        cfg: Some(ProjectSourceCfg {
                            git: Some(GitProjectSource {
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
                    if let Ok(ProjectSourceStatus { state, reason, .. }) = entry {
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
            }
        }

        Ok(())
    }
}
