use crate::{Error, Result};
use ame_client::TaskLogRequest;
use ame_client::{
    ame_service_client::AmeServiceClient, project_file_chunk::Messages, CreateTaskRequest,
    FileChunk, ProjectFileChunk, ProjectFileIdentifier, TaskProjectDirectoryStructure,
    TaskTemplate,
};
use console::Emoji;
use futures_util::StreamExt;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use tonic::transport::Channel;

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Project {
    #[serde(rename = "projectid")]
    id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    tasks: Option<Vec<TaskTemplate>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    templates: Option<Vec<TaskTemplate>>,
}

impl Project {
    pub fn new(name: &str) -> Self {
        Project {
            id: name.to_string(),
            ..Project::default()
        }
    }

    pub fn init(name: &str) -> Result<()> {
        if File::open("ame.yaml").is_ok() {
            println!(
                "An AME project is already initiated in this directory. {}",
                Emoji("✨", ":-)")
            );

            Ok(())
        } else {
            let project = Project::new(name);
            let f = File::create("ame.yaml")?;
            serde_yaml::to_writer(f, &project).unwrap();

            println!("Project initialised {}", Emoji("🏗", ""));

            Ok(())
        }
    }

    pub fn init_from_working_dir() -> Result<Self> {
        Ok(serde_yaml::from_str(&fs::read_to_string("ame.yaml")?)?)
    }

    pub fn get_task_template(&self, name: &str) -> Result<TaskTemplate> {
        let Some(task_templates) = self.tasks.clone() else {
            return Err(Error::EmptyProjectField("tasks".to_string()));
        };

        let valid_task_templates: Vec<&TaskTemplate> =
            task_templates.iter().filter(|t| &t.name == name).collect();

        if valid_task_templates.len() > 1 {
            return Err(Error::MisConfiguredProject(
                "found multiple tasks templates with the same name".to_string(),
            ));
        }

        if valid_task_templates.len() == 0 {
            return Err(Error::MissingTaskTemplate(name.to_string()));
        }

        Ok(valid_task_templates[0].clone())
    }

    pub async fn run_task(
        &self,
        mut client: AmeServiceClient<Channel>,
        template_name: &str,
    ) -> Result<()> {
        let project_file: Project = serde_yaml::from_str(&fs::read_to_string("ame.yaml")?)?;
        let task_template = project_file.get_task_template(&template_name)?;

        // TODO: handle name clashes in the cluster.
        let random_task_name = format!(
            "{}{}",
            template_name,
            Alphanumeric.sample_string(&mut rand::thread_rng(), 6)
        )
        .to_lowercase();

        client
            .create_task_project_directory(tonic::Request::new(TaskProjectDirectoryStructure::new(
                &random_task_name,
                &self.id,
                vec![],
            )))
            .await?;

        let _chunk_size = 1024;

        for entry in walkdir::WalkDir::new(".").into_iter().flatten() {
            let Ok(mut f) = File::open(entry.path()) else {
                            continue;
                        };

            let mut buf: [u8; 100] = [0; 100];

            // TODO: How do we test that files are uploaded transferred correctly? in the
            // common ame-client library perhaps?
            let stre = async_stream::stream! {

                yield ProjectFileChunk {
                    messages: Some(Messages::Identifier(ProjectFileIdentifier{
                        taskid: "testtask".to_string(),
                        filepath:entry.path().to_str().unwrap().to_string(),
                    }))
                };

                loop {

            let n = match f.read(&mut buf) {
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

        println!("uploaded project!");

        client
            .create_task(tonic::Request::new(CreateTaskRequest::new(
                &random_task_name,
                task_template,
            )))
            .await?;

        tokio::time::sleep(Duration::from_secs(1)).await;

        let mut log_stream = client
            .stream_task_logs(tonic::Request::new(TaskLogRequest::stream_from_beginning(
                &random_task_name,
                true,
            )))
            .await?
            .into_inner();

        while let Some(entry) = log_stream.next().await {
            println!("{}", String::from_utf8(entry?.contents).unwrap());
        }

        Ok(())
    }
}
