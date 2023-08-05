use std::cmp::max;

use ame::{
    client::native_client::{build_ame_client, AmeClient},
    grpc::{
        CreateProjectRequest, ListTasksRequest, ProjectCfg, RunTaskRequest, TaskIdentifier,
        TaskLogRequest,
    },
    AmeServiceClientCfg,
};
use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use console::Term;
use dialoguer::theme::ColorfulTheme;
use futures_util::StreamExt;
use spinners::Spinner;
use tokio::{fs::File, io::AsyncReadExt};
use tonic::Request;

use crate::CliConfiguration;
use ame::grpc::{project_file_chunk::Messages, FileChunk, ProjectFileChunk, ProjectFileIdentifier};
use dialoguer::FuzzySelect;

/// Manage Tasks
#[derive(Subcommand)]
pub enum TaskCommand {
    /// Run a Task present from your local project on a remote `AME` instance.
    ///
    /// Note: This command assumes that it is run from the root of an `AME` project directory.
    /// Note: Every file in your project directory is uploaded. Options for ignoring files
    /// will be implemented in the very near future, see https://github.com/TeaInSpace/ame/issues/151.
    ///
    /// This command functions in three steps:
    /// 1. Upload your local project configuration from `ame.yaml` to `AME`. This will appear as project with your
    ///    local machine as the source.
    ///
    /// 2. Upload the files from the project directory to `AME's` object.
    ///    Note that all files will be uploaded.  
    ///
    /// 3. Start the chosen Task with the uploaded Project and files as the context.
    #[clap(verbatim_doc_comment)]
    Run {
        /// Name of the Task to run.
        name: Option<String>,

        /// Stream logs live while the Task runs.
        #[clap(long)]
        logs: bool,
    },

    /// Stream the logs a completed or running Task.
    ///
    /// Logs for a running Task will be streamed until the Task
    /// stops.
    Logs {
        /// Name of Task to run.
        name: Option<String>,
    },

    /// List Tasks
    ///
    /// Note that this command list Tasks present in the `AME` instance, not in your `AME` file.
    List {},

    /// Remove a Task from the `AME` instance.
    ///
    /// Note that this is a destructive action and can not be recovered, use with care.
    ///
    /// If the Task is not approved for deletion removal will be blocked.
    Remove {
        name: Option<String>,

        /// Automatically approve deletion, this can be very destructive use with care!
        #[clap(long)]
        approve: bool,
    },

    /// View the configuration for a Task
    View { name: Option<String> },
}

pub async fn select_task(client: &mut AmeClient) -> Result<String> {
    let tasks = client
        .list_tasks(ListTasksRequest {})
        .await
        .map_err(|e| crate::Error::from(e))?
        .into_inner()
        .tasks;

    let task_table: Vec<String> = tasks
        .clone()
        .into_iter()
        .map(|(k, v)| format!("{k} {}", v.time_stamp))
        .collect();

    let task_names: Vec<String> = tasks.into_keys().collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .items(&task_table)
        .default(0)
        .interact_on_opt(&Term::stderr())?
        .unwrap();

    Ok(task_names[selection].clone())
}

pub async fn logs(mut client: AmeClient) -> Result<()> {
    let tasks = client
        .list_tasks(ListTasksRequest {})
        .await
        .map_err(|e| crate::Error::from(e))?
        .into_inner()
        .tasks;

    let task_table: Vec<String> = tasks
        .clone()
        .into_iter()
        .map(|(k, v)| format!("{k} {}", v.time_stamp))
        .collect();

    let task_names: Vec<String> = tasks.into_keys().collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .items(&task_table)
        .default(0)
        .interact_on_opt(&Term::stderr())?
        .unwrap();

    let task_name = task_names[selection].clone();

    let mut log_stream = client
        .stream_task_logs(tonic::Request::new(TaskLogRequest::stream_from_beginning(
            &task_name, true,
        )))
        .await?
        .into_inner();

    while let Some(entry) = log_stream.next().await {
        let Ok(line) = String::from_utf8(entry.clone()?.contents) else {
            println!("failed to parse log entry: {entry:?}");
            return Ok(());
        };

        print!("{line}");
    }

    Ok(())
}

async fn exec_task_rm(
    mut client: AmeClient,
    name: Option<String>,
    approve: Option<bool>,
) -> Result<()> {
    let task_name = if let Some(name) = name {
        name
    } else {
        select_task(&mut client).await?
    };

    client
        .remove_task(Request::new(ame::grpc::RemoveTaskRequest {
            name: task_name.clone(),
            approve,
        }))
        .await?;

    println!("{task_name} {}", "Deleted".red().bold());

    return Ok(());
}

struct Table {
    rows: Vec<Vec<String>>,
    sort: bool,
}

impl Table {
    pub fn new(header: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let mut row = vec![header];
        row.extend(rows);

        Self {
            rows: row,
            sort: false,
        }
    }

    fn sort(&mut self, sort: bool) -> &mut Self {
        self.sort = sort;
        self
    }

    fn row_len(&self) -> usize {
        self.rows.get(0).map(|r| r.len()).unwrap_or(0)
    }

    pub fn try_string_colored(&self) -> Result<String> {
        if self.rows.iter().any(|row| (row.len() != self.row_len())) {
            todo!("error");
        }

        let widths: Vec<usize> =
            self.rows
                .iter()
                .fold(vec![0 as usize; self.row_len()], |acc, r| {
                    acc.iter()
                        .zip(r.iter())
                        .map(|(acc_l, v)| max(*acc_l, v.chars().count()))
                        .collect()
                });

        let headers: String = self.rows[0]
            .iter()
            .zip(widths.clone())
            .map(|(v, width)| {
                format!(
                    "{}{} ",
                    v.white().bold(),
                    vec![" "; width - v.chars().count()].join("")
                )
            })
            .collect();

        let mut rows = self.rows.as_slice().split_first().unwrap().1.to_owned();

        if self.sort {
            rows.sort_by_key(|row| row[0].clone());
        }

        let rows: String = rows.iter().fold("\n".to_string(), |acc, row| {
            acc + &row
                .iter()
                .zip(widths.clone())
                .map(|(v, width)| {
                    format!("{}{} ", v, vec![" "; width - v.chars().count()].join(""))
                })
                .collect::<String>()
                + "\n"
        });

        Ok(headers + &rows)
    }
}

async fn exec_task_view(mut client: AmeClient, name: Option<String>) -> Result<()> {
    let task_name = if let Some(name) = name {
        name
    } else {
        select_task(&mut client).await?
    };

    let task_info = client
        .get_task(Request::new(TaskIdentifier { name: task_name }))
        .await?
        .into_inner();

    println!("{}", serde_yaml::to_string(&task_info)?);

    Ok(())
}

async fn exec_task_list(mut client: AmeClient) -> Result<()> {
    let tasks = client.list_tasks(Request::new(ListTasksRequest {})).await?;
    let _widths: Vec<usize> = vec![0, 0];

    let mut table = Table::new(
        vec![
            "Name".to_string(),
            "Status".to_string(),
            "Project".to_string(),
        ],
        tasks
            .into_inner()
            .tasks
            .iter()
            .map(|t| {
                vec![
                    t.0.to_string(),
                    t.1.status
                        .as_ref()
                        .unwrap()
                        .phase
                        .as_ref()
                        .unwrap()
                        .to_string(),
                    "Unknown".to_string(),
                ]
            })
            .collect(),
    );

    table.sort(true);

    println!("{}", table.try_string_colored()?);

    return Ok(());
}

pub async fn exec_task_command(cfg: CliConfiguration, cmd: &TaskCommand) -> Result<()> {
    let mut client = build_ame_client(AmeServiceClientCfg {
        disable_tls_cert_check: true,
        endpoint: cfg.endpoint.parse().unwrap(),
        id_token: cfg.id_token,
    })
    .await?;

    match cmd {
        TaskCommand::Logs { name: _ } => {
            return logs(client).await;
        }
        TaskCommand::List {} => {
            return exec_task_list(client).await;
        }
        TaskCommand::Remove { name, approve } => {
            return exec_task_rm(client, name.to_owned(), Some(*approve)).await;
        }
        TaskCommand::View { name } => {
            return exec_task_view(client, name.to_owned()).await;
        }
        _ => (),
    };

    let project = ProjectCfg::try_from_working_dir()?;

    let (task_cfg, display_logs) = if let TaskCommand::Run {
        name: Some(ref name),
        logs: display_logs,
    } = cmd
    {
        // TODO: migrate CLI code to use common error type.
        (project.get_task_cfg(name).unwrap(), *display_logs)
    } else {
        let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
            .items(&project.task_names())
            .default(0)
            .interact_on_opt(&Term::stderr())?
            .unwrap();

        (
            project
                .get_task_cfg(&project.task_names()[selection])
                .unwrap(),
            false,
        )
    };

    let is_tty = atty::is(atty::Stream::Stdout);

    if is_tty {
        let _spinner = Spinner::new(
            spinners::Spinners::Dots,
            format!(" {} project", "Uploading".cyan().bold()),
        );
    } else {
        println!("Uploading project!")
    }

    let project_id = client
        .create_project(CreateProjectRequest {
            cfg: Some(project),
            enable_triggers: Some(false),
        })
        .await?
        .into_inner();

    let task_name = task_cfg.clone().name.unwrap();

    let _chunk_size = 500;

    for entry in walkdir::WalkDir::new(".").into_iter().flatten() {
        let project_id = project_id.clone();
        if entry.metadata()?.is_dir() {
            continue;
        }

        let Ok(mut f) = File::open(entry.clone().path()).await else {
            continue;
        };
        println!("Uploading file: {}", entry.clone().path().to_str().unwrap());

        let _task_name = task_name.clone();

        let mut buf: [u8; 1000] = [0; 1000];

        // TODO: How do we test that files are uploaded transferred correctly? in the
        // common ame-client library perhaps?
        let stre = async_stream::stream! {

            // TODO: using the taskid needs to be changed to project id.
            yield ProjectFileChunk {
                messages: Some(Messages::Identifier(ProjectFileIdentifier{
                    taskid: project_id.name.clone(),
                    filepath:entry.clone().path().to_str().unwrap().to_string(),
                }))
            };

            loop {

        let n = match f.read(&mut buf).await {
            Ok(0) => {
                break;
            }

            Ok(n) => n,

            // TODO: how do we handle errors here?
            Err(_) => {
                break;
            },
        };

        yield  ProjectFileChunk {
            messages: Some(Messages::Chunk(FileChunk {
                 contents: buf.get(0..n).unwrap().to_vec()
            })),
         };

        buf = [0; 1000];
         }

            };

        client.upload_project_file(stre).await?;
    }

    let task_id = client
        .run_task(RunTaskRequest {
            project_id: Some(project_id),
            task_cfg: Some(task_cfg),
        })
        .await?;

    if display_logs {
        let mut log_stream = client
            .stream_task_logs(tonic::Request::new(TaskLogRequest::stream_from_beginning(
                &task_id.into_inner().name,
                true,
            )))
            .await?
            .into_inner();

        while let Some(entry) = log_stream.next().await {
            // TODO: What to do with errors here instead of default?
            let Ok(line) = String::from_utf8(entry.clone().unwrap_or_default().contents) else {
                println!("failed to parse log entry: {entry:?}");
                return Ok(());
            };

            print!("{line}");
        }
    }

    // TODO: handle ignoring large files.

    Ok(())
    //    todo!()
}
