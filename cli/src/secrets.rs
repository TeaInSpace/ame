use std::time::Duration;

use ame_client::client_builder::{build_ame_client, AmeServiceClientCfg};
use ame_client::Empty;
use ame_client::{AmeSecret, AmeSecretId};
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Password};
use spinners::Spinner;
use tonic::Request;

use crate::{CliConfiguration, Result};

#[derive(Subcommand)]
pub enum SecretCommand {
    Create {
        key: Option<String>,
        #[arg(short, long)]
        value: Option<String>,
    },

    Delete {
        key: Option<String>,
    },

    List,
}

pub async fn exec_secret_command(cfg: CliConfiguration, cmd: &SecretCommand) -> Result<()> {
    let mut client = build_ame_client(AmeServiceClientCfg {
        disable_tls_cert_check: true,
        endpoint: cfg.endpoint.parse().unwrap(),
        id_token: cfg.id_token,
    })
    .await?;

    match cmd {
        SecretCommand::Create { key, value } => {
            let secret = AmeSecret {
                key: key.clone().unwrap_or_else(|| {
                    Input::new()
                        .with_prompt("Please provide the secret key")
                        .interact()
                        .unwrap()
                }),
                value: value.clone().unwrap_or_else(|| {
                    Password::new()
                        .with_prompt("Please provide the secret value")
                        .interact()
                        .unwrap()
                }),
            };

            let mut spinner = Spinner::new(
                spinners::Spinners::Dots,
                format!("{} secret", "Storing".cyan().bold()),
            );

            let mut request = Request::new(secret);
            request.set_timeout(Duration::from_secs(5));

            let res = client.create_secret(request).await;

            match res {
                Ok(_) => {
                    spinner.stop_and_persist(" ", format!("{} secret", "Stored".green().bold()));
                    Ok(())
                }
                Err(e) => {
                    spinner.stop_and_persist(
                        " ",
                        format!("{} to store secret", "Failed".red().bold()),
                    );
                    Err(e)
                }
            }
        }
        SecretCommand::Delete { key } => {
            let secret_id = AmeSecretId {
                key: key.clone().unwrap_or_else(|| {
                    Input::new()
                        .with_prompt("Please provide a secret key")
                        .interact()
                        .unwrap()
                }),
            };

            let mut spinner = Spinner::new(
                spinners::Spinners::Dots,
                format!("{} secret", "Deleting".cyan().bold()),
            );

            match client.delete_secret(Request::new(secret_id)).await {
                Ok(_) => {
                    spinner.stop_and_persist(" ", format!("{} secret", "Deleted".green().bold()));
                    Ok(())
                }
                Err(e) => {
                    spinner.stop_and_persist(
                        " ",
                        format!("{} to delete secret", "Failed".red().bold()),
                    );
                    Err(e)
                }
            }
        }
        SecretCommand::List => {
            let mut spinner = Spinner::new(
                spinners::Spinners::Dots,
                format!("{} secrets", "Fetching".cyan().bold()),
            );
            let secrets = client.list_secrets(Request::new(Empty {})).await?;
            spinner.stop_with_message(format!("{} ", "Key".white().bold()));

            for s in secrets.into_inner().secrets {
                println!("{}", s.key);
            }

            Ok(())
        }
    }?;
    Ok(())
}
