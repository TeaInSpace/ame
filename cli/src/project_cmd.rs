use ame::{
    client::native_client::build_ame_client,
    grpc::{
        project_file_chunk::Messages, CreateProjectRequest, FileChunk, ProjectCfg,
        ProjectFileChunk, ProjectFileIdentifier,
    },
    AmeServiceClientCfg,
};
use clap::Subcommand;

use tokio::{fs::File, io::AsyncReadExt};

use crate::CliConfiguration;
use anyhow::Result;

#[derive(Subcommand)]
pub enum ProjectCommands {
    Push {
        #[clap(short, long)]
        triggers: bool,
    },
    Delete,
}

pub async fn exec_project_command(cfg: CliConfiguration, cmd: &ProjectCommands) -> Result<()> {
    let mut client = build_ame_client(AmeServiceClientCfg {
        disable_tls_cert_check: true,
        endpoint: cfg.endpoint.parse().unwrap(),
        id_token: cfg.id_token,
    })
    .await?;

    let triggers = if let ProjectCommands::Push { triggers } = cmd {
        *triggers
    } else {
        false
    };

    let project = ProjectCfg::try_from_working_dir()?;

    let project_id = client
        .create_project(CreateProjectRequest {
            cfg: Some(project),
            enable_triggers: Some(triggers),
        })
        .await?
        .into_inner();

    let _chunk_size = 500;

    let _is_tty = atty::is(atty::Stream::Stdout);

    println!("Uploading project!");

    for entry in walkdir::WalkDir::new(".").into_iter().flatten() {
        let project_id = project_id.clone();
        if entry.metadata()?.is_dir() {
            continue;
        }

        let Ok(mut f) = File::open(entry.clone().path()).await else {
            continue;
        };

        println!("Uploading file: {}", entry.clone().path().to_str().unwrap());

        let mut buf: [u8; 100] = [0; 100];

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

        buf = [0; 100];
         }

            };

        client.upload_project_file(stre).await?;
    }

    println!("Done!");

    Ok(())
}
