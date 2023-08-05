use ame::client::native_client::AmeClient;
use anyhow::Result;

use ame::grpc::TaskCfg;
use console::Emoji;

use serde::{Deserialize, Serialize};
use std::{fs, fs::File};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Project {
    #[serde(rename = "name")]
    id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    tasks: Option<Vec<TaskCfg>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    templates: Option<Vec<TaskCfg>>,
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

    pub fn task_names(&self) -> Vec<String> {
        self.tasks
            .as_ref()
            .map(|tasks| tasks.iter().filter_map(|t| t.name.clone()).collect())
            .unwrap_or(vec![])
    }
    pub fn init_from_working_dir() -> Result<Self> {
        Ok(serde_yaml::from_str(&fs::read_to_string("ame.yaml")?)?)
    }

    pub fn get_task_template(&self, _name: &str) -> Result<TaskCfg> {
        todo!();
        /*
        let valid_task_templates: Vec<&TaskTemplate> =
            task_templates.iter().filter(|t| t.name == name).collect();

        if valid_task_templates.len() > 1 {
            return Err(Error::MissConfiguredProject(
                "found multiple tasks templates with the same name".to_string(),
            ));
        }

        if valid_task_templates.is_empty() {
            return Err(Error::MissingTaskTemplate(name.to_string()));
        }

        Ok(valid_task_templates[0].clone())
        */
    }

    pub async fn run_task(&self, _client: &mut AmeClient, template_name: &str) -> Result<()> {
        let project_file: Project = serde_yaml::from_str(&fs::read_to_string("ame.yaml")?)?;
        let _task_template = project_file.get_task_template(template_name)?;
        Ok(())

        //     // TODO: handle name clashes in the cluster.
        //     let random_task_name = format!(
        //         "{}{}",
        //         template_name,
        //         Alphanumeric.sample_string(&mut rand::thread_rng(), 6)
        //     )
        //     .to_lowercase();

        //     client
        //         .create_task_project_directory(tonic::Request::new(TaskProjectDirectoryStructure::new(
        //             &random_task_name,
        //             &self.id,
        //             vec![],
        //         )))
        //         .await?;

        //     let _chunk_size = 500;

        //     for entry in walkdir::WalkDir::new(".").into_iter().flatten() {
        //         if entry.metadata()?.is_dir() {
        //             continue;
        //         }

        //         let task_name = random_task_name.clone();

        //         let Ok(mut f) = File::open(entry.clone().path()) else {
        //                         continue;
        //                     };

        //         let mut buf: [u8; 100] = [0; 100];

        //         // TODO: How do we test that files are uploaded transferred correctly? in the
        //         // common ame-client library perhaps?
        //         let stre = async_stream::stream! {

        //             yield ProjectFileChunk {
        //                 messages: Some(Messages::Identifier(ProjectFileIdentifier{
        //                     taskid: task_name,
        //                     filepath:entry.clone().path().to_str().unwrap().to_string(),
        //                 }))
        //             };

        //             loop {

        //         let n = match f.read(&mut buf) {
        //             Ok(0) => {
        //                 break;
        //             }

        //             Ok(n) => n,

        //             // TODO: how do we handle errors here?
        //             Err(_) => {
        //                 break;
        //             },
        //         };

        //         yield  ProjectFileChunk {
        //             messages: Some(Messages::Chunk(FileChunk {
        //                  contents: buf.get(0..n).unwrap().to_vec()
        //             })),
        //          };

        //         buf = [0; 100];
        //          }

        //             };

        //         client.upload_project_file(stre).await?;
        //     }

        //     println!("uploaded project!");

        //     client
        //         .create_task(tonic::Request::new(CreateTaskRequest::new(
        //             &random_task_name,
        //             task_template,
        //         )))
        //         .await?;

        //     tokio::time::sleep(Duration::from_secs(1)).await;

        //     let mut log_stream = client
        //         .stream_task_logs(tonic::Request::new(TaskLogRequest::stream_from_beginning(
        //             &random_task_name,
        //             true,
        //         )))
        //         .await?
        //         .into_inner();

        //     while let Some(entry) = log_stream.next().await {
        //         let Ok(line) = String::from_utf8(entry.clone()?.contents) else {
        //             println!("failed to parse log entry: {entry:?}");
        //             return Ok(());
        //         };

        //         if line.contains("s3") || line.contains("argo") {
        //             continue;
        //         }

        //         if line.contains("WARNING:") {
        //             break;
        //         }

        //         print!("{line}");
        //     }

        //     Ok(())
    }
}
