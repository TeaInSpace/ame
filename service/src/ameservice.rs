use crate::{Error, Result};
use kube::core::ObjectMeta;

tonic::include_proto!("ame.v1");

use controller::manager::TaskPhase;
use futures_util::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{DeleteParams, ListParams, LogParams, PostParams};
use kube::runtime::wait::await_condition;
use kube::runtime::wait::conditions;
use kube::{Api, Client, ResourceExt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;

use ame_service_server::AmeService;
use project_file_chunk::Messages;

use crate::storage::{AmeFile, ObjectStorage, S3Config, S3StorageDriver};
use tracing::{debug, info};

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tonic::{Request, Response, Status, Streaming};

#[derive(Debug, Clone)]
pub struct AmeServiceConfig {
    pub s3config: S3Config,
    pub bucket: String,
}

#[derive(Debug, Clone)]
pub struct Service {
    tasks: Arc<Api<controller::Task>>,
    project_srcs: Arc<Api<controller::ProjectSource>>,
    projects: Arc<Api<controller::Project>>,
    storage: Arc<ObjectStorage<S3StorageDriver>>,
    pods: Arc<Api<Pod>>,
}

#[tonic::async_trait]
impl AmeService for Service {
    async fn train_model(&self, request: Request<TrainRequest>) -> Result<Response<Empty>, Status> {
        let tr = request.into_inner();

        let Some(project) = self.projects.list(&ListParams::default()).await.unwrap().into_iter().find(|p| p.spec.id == tr.projectid) else {
            return Err(Status::from_error(Box::new(Error::MissingModel(tr.model_name.clone()))));
        };

        self.tasks
            .create(
                &PostParams::default(),
                &project
                    .generate_model_training_task(&tr.model_name)
                    .unwrap(),
            )
            .await
            .unwrap();

        Ok(Response::new(Empty {}))
    }

    async fn get_task(
        &self,
        request: Request<TaskIdentifier>,
    ) -> Result<Response<TaskTemplate>, Status> {
        let task = self
            .tasks
            .get(&request.get_ref().name)
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(Response::new(TaskTemplate::from(task)))
    }

    async fn create_task(
        &self,
        request: Request<CreateTaskRequest>,
    ) -> Result<Response<TaskIdentifier>, Status> {
        let task_in_cluster = self
            .tasks
            .create(
                &PostParams::default(),
                &controller::Task::try_from(request.into_inner())
                    .map_err(|e| Status::from_error(Box::new(e)))?,
            )
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(Response::new(TaskIdentifier::from(task_in_cluster)))
    }

    async fn delete_task(
        &self,
        request: Request<TaskIdentifier>,
    ) -> Result<Response<Empty>, Status> {
        self.tasks
            .delete(&request.get_ref().name, &DeleteParams::default())
            .await
            .map_or_else(
                |e| Err(Status::from_error(Box::new(e))),
                |_| Ok(Response::new(Empty {})),
            )
    }

    async fn create_task_project_directory(
        &self,
        request: Request<TaskProjectDirectoryStructure>,
    ) -> Result<Response<Empty>, Status> {
        let structure = request.into_inner();
        let TaskProjectDirectoryStructure{taskid: Some(task_id), ..} = structure.clone() else {
            return Err(Status::invalid_argument("missing Task identifier"))
        };

        self.storage
            .store_project_directory_structure(&task_id, structure)
            .await
            .map_or_else(
                |e| Err(Status::from_error(Box::new(e))),
                |_| Ok(Response::new(Empty {})),
            )
    }

    async fn upload_project_file(
        &self,
        request: Request<Streaming<ProjectFileChunk>>,
    ) -> Result<Response<Empty>, Status> {
        let mut file: AmeFile = AmeFile::default();
        let mut task_id_option: Option<TaskIdentifier> = None;

        let mut stream = request.into_inner();
        loop {
            match stream.message().await {
                Ok(Some(ProjectFileChunk {
                    messages: Some(Messages::Chunk(mut chunk)),
                })) => file.contents.append(&mut chunk.contents),
                Ok(Some(ProjectFileChunk {
                    messages: Some(Messages::Identifier(id)),
                })) => {
                    file.key = id.filepath;
                    task_id_option = Some(TaskIdentifier { name: id.taskid });
                }
                Ok(Some(ProjectFileChunk { messages: None })) => {
                    return Err(Status::invalid_argument(
                        "missing messages from ProjectFileChunk",
                    ))
                }
                Err(e) => {
                    return Err(Status::cancelled(format!(
                        "Stream was cancelled by the caller, with status: {e}"
                    )))
                }
                Ok(None) => break,
            }
        }

        let Some(task_id) =  task_id_option  else {
            return Err(Status::invalid_argument("missing TaskIdentifier in stream"));
        };

        if file.key.is_empty() {
            return Err(Status::invalid_argument(
                "missing ProjectFileIdentifier in stream",
            ));
        }

        //TODO: is an empty file valid?

        self.storage
            .write_task_project_file(&task_id, file)
            .await
            .map_or_else(
                |e| Err(Status::from_error(Box::new(e))),
                |_| Ok(Response::new(Empty {})),
            )
    }

    type StreamTaskLogsStream = ReceiverStream<Result<LogEntry, Status>>;

    async fn stream_task_logs(
        &self,
        request: Request<TaskLogRequest>,
    ) -> Result<Response<Self::StreamTaskLogsStream>, Status> {
        let (log_sender, log_receiver) = mpsc::channel(1);
        let task_name = request.get_ref().clone().taskid.unwrap().name;
        let pods = self.pods.clone();
        let tasks = self.tasks.clone();

        tracing::info!("streaming logs for: {}", &task_name);
        let _handle: JoinHandle<Result<()>> = tokio::spawn(async move {
            loop {
                let task_pods = pods
                    .list(&ListParams::default().labels(&format!("ame-task={task_name}")))
                    .await
                    .unwrap();

                // TODO: ensure the pod is actually done before  giving up on logging.
                if !task_pods.items.is_empty() {
                    let pod = &task_pods.items[0];
                    info!("found pod {} for task {}", &pod.name_any(), &task_name);

                    for pod in &task_pods.items {
                        let status = pods.get_status(&pod.name_any()).await.unwrap();
                        if status.status.unwrap().phase.unwrap() != "Running" {
                            continue;
                        }
                        await_condition(
                            Api::<Pod>::clone(&pods),
                            &pod.name_any(),
                            conditions::is_pod_running(),
                        )
                        .await?;

                        info!("pod is running!");

                        let mut pod_log_stream = pods
                            .log_stream(
                                &pod.name_any(),
                                &LogParams {
                                    container: Some("main".to_string()),
                                    follow: true,
                                    since_seconds: Some(1),

                                    ..LogParams::default()
                                },
                            )
                            .await?;

                        while let Some(e) = pod_log_stream.next().await {
                            debug!("sent log entry: {:?}", &e);
                            let log_entry = LogEntry {
                                contents: e.unwrap().to_vec(),
                            };
                            log_sender.send(Ok(log_entry.clone())).await.or(Err(
                                Error::TokioSendError(format!(
                                    "failed to send log entry: {}",
                                    String::from_utf8(log_entry.contents)?
                                )),
                            ))?
                        }
                    }
                }

                let Ok(task) = tasks
                    .get_status(&task_name)
                    .await else {
                        sleep(Duration::from_secs(1)).await;
                        continue;
                    };

                if task.status.clone().unwrap().phase.unwrap() != TaskPhase::Running
                    && task.status.unwrap().phase.unwrap() != TaskPhase::Pending
                {
                    return Ok(());
                }

                sleep(Duration::from_millis(50)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(log_receiver)))
    }

    async fn create_project_src(
        &self,
        request: Request<ProjectSource>,
    ) -> Result<Response<Empty>, Status> {
        self.project_srcs
            .create(
                &PostParams::default(),
                &controller::ProjectSource::try_from(request.into_inner())
                    .map_err(|e| Status::from_error(Box::new(e)))?,
            )
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(Response::new(Empty {}))
    }
}

impl Service {
    /// This method initializes a Service with the required clients and configuration.
    /// It will fail if a `Kubernetes` cluster is not reachable.
    pub async fn try_init(cfg: AmeServiceConfig) -> Result<Service> {
        let client = Client::try_default().await?;
        let target_namespace = "ame-system";
        let tasks = Api::<controller::Task>::namespaced(client.clone(), target_namespace);
        let pods = Api::<Pod>::namespaced(client.clone(), target_namespace);
        let project_srcs =
            Api::<controller::ProjectSource>::namespaced(client.clone(), target_namespace);
        let projects = Api::<controller::Project>::namespaced(client, target_namespace);

        let task_service = Service {
            tasks: Arc::new(tasks),
            pods: Arc::new(pods),
            project_srcs: Arc::new(project_srcs),
            projects: Arc::new(projects),
            storage: Arc::new(ObjectStorage::<S3StorageDriver>::new_s3_storage(
                &cfg.bucket,
                cfg.s3config,
            )?),
        };

        Ok(task_service)
    }

    pub async fn prepare_environment(&self) -> Result<()> {
        match self.storage.ensure_storage_is_ready().await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("failed to prepare object storage: {}", e);
                Err(e)
            }
        }
    }
}

impl TryFrom<self::CreateTaskRequest> for controller::Task {
    type Error = Error;

    fn try_from(t: CreateTaskRequest) -> Result<Self> {
        let CreateTaskRequest {
            id: Some(TaskIdentifier { name: id }),
            template: Some(template),
        } = t else {
            return Err(Error::ConversionError("Failed to extract id and template from CreateTaskRequest".to_string()))
        };

        Ok(controller::Task {
            metadata: ObjectMeta {
                name: Some(id),
                ..ObjectMeta::default()
            },
            spec: controller::TaskSpec {
                projectid: Some(template.projectid),
                runcommand: Some(template.command),
                image: template.image,
                task_type: template.task_type.map(|t| {
                    if t == 1 {
                        controller::TaskType::Mlflow
                    } else {
                        controller::TaskType::PipEnv
                    }
                }),
                ..controller::TaskSpec::default()
            },
            status: None,
        })
    }
}

impl TryFrom<ProjectSource> for controller::ProjectSource {
    type Error = Error;

    fn try_from(project_src: ProjectSource) -> std::result::Result<Self, Self::Error> {
        let Some(GitProjectSource {
                repository,
                username,
                secret,
                sync_interval: _,
                ..
            }) = project_src.git else {
                return Err(Error::ConversionError("Could not generate ProjectSource object".to_string()));
            };
        Ok(controller::ProjectSource {
            metadata: ObjectMeta {
                generate_name: Some("ameprojectsrc".to_string()),
                ..ObjectMeta::default()
            },
            spec: controller::ProjectSourceSpec {
                git: Some(controller::GitProjectSource {
                    repository,
                    username,
                    secret,
                    sync_interval: None,
                }),
            },
            status: None,
        })
    }
}

impl From<TaskTemplate> for controller::Task {
    fn from(t: TaskTemplate) -> Self {
        controller::Task {
            metadata: ObjectMeta {
                generate_name: Some("mytask".to_string()),
                ..ObjectMeta::default()
            },
            spec: controller::TaskSpec {
                projectid: Some(t.projectid),
                runcommand: Some(t.command),
                image: t.image,
                task_type: t.task_type.map(|t| {
                    if t == 1 {
                        controller::TaskType::Mlflow
                    } else {
                        controller::TaskType::PipEnv
                    }
                }),
                ..controller::TaskSpec::default()
            },
            status: None,
        }
    }
}

impl From<controller::Task> for TaskTemplate {
    fn from(t: controller::Task) -> Self {
        TaskTemplate {
            name: "".to_string(),
            command: t.spec.runcommand.unwrap_or("".to_string()),
            projectid: t.spec.projectid.unwrap_or("".to_string()),
            image: t.spec.image,
            task_type: t.spec.task_type.map(|t| {
                if t == controller::TaskType::Mlflow {
                    1
                } else {
                    0
                }
            }),
        }
    }
}

impl From<controller::Task> for TaskIdentifier {
    fn from(t: controller::Task) -> Self {
        TaskIdentifier { name: t.name_any() }
    }
}
