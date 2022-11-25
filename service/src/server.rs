use controller::manager::{TaskPhase, TaskStatus};
use futures_util::StreamExt;
use futures_util::TryStreamExt;
use hyper::header;
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{DeleteParams, ListParams, LogParams, PostParams};
use kube::runtime::wait::await_condition;
use kube::runtime::wait::conditions;
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, Client, ResourceExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic_health::server::HealthReporter;

mod ameservice;
use ameservice::ame_service_server::{AmeService, AmeServiceServer};
use ameservice::{
    project_file_chunk::Messages, CreateTaskRequest, Empty, LogEntry, ProjectFileChunk,
    TaskIdentifier, TaskLogRequest, TaskProjectDirectoryStructure, TaskTemplate,
};

mod storage;
use storage::{AmeFile, ObjectStorage, S3Config, S3StorageDriver};
use tracing::debug;

use std::iter::once;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tonic::transport::{NamedService, Server};
use tonic::{Request, Response, Status, Streaming};
use tower::ServiceBuilder;
use tower_http::{
    classify::{GrpcCode, GrpcErrorsAsFailures, SharedClassifier},
    compression::CompressionLayer,
    sensitive_headers::SetSensitiveHeadersLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, TraceLayer},
};

use ameservice_public::{Error, Result};
use envconfig::Envconfig;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

use tracing::Level;

impl TryFrom<CreateTaskRequest> for controller::Task {
    type Error = Error;

    fn try_from(t: CreateTaskRequest) -> Result<Self> {
        let CreateTaskRequest {
            id: Some(TaskIdentifier { name: id }),
            templat: Some(template),
        } = t else {
            return Err(Error::ConversionError("Failed to extract id and template from CreateTaskRequest".to_string()))
        };

        Ok(controller::Task {
            metadata: ObjectMeta {
                name: Some(id),
                ..ObjectMeta::default()
            },
            spec: controller::TaskSpec {
                projectid: template.projectid,
                runcommand: template.command,
                image: template.image,
                ..controller::TaskSpec::default()
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
                projectid: t.projectid,
                runcommand: t.command,
                image: t.image,
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
            command: t.spec.runcommand,
            projectid: t.spec.projectid,
            image: t.spec.image,
        }
    }
}

impl From<controller::Task> for TaskIdentifier {
    fn from(t: controller::Task) -> Self {
        TaskIdentifier { name: t.name_any() }
    }
}

#[derive(Debug, Clone)]
struct AmeServiceConfig {
    s3config: S3Config,
    bucket: String,
}

#[derive(Debug, Clone)]
struct AService {
    tasks: Arc<Api<controller::Task>>,
    storage: Arc<ObjectStorage<S3StorageDriver>>,
    pods: Arc<Api<Pod>>,
}

#[tonic::async_trait]
impl AmeService for AService {
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
                        "Stream was cancelled by the caller, with status: {}",
                        e
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
        let (mut log_sender, log_reciever) = mpsc::channel(1);
        let task_name = request.get_ref().clone().taskid.unwrap().name;
        let pods = self.pods.clone();
        let tasks = self.tasks.clone();

        debug!("streaming logs for: {}", &task_name);
        tokio::spawn(async move {
            loop {
                let task_pods = pods
                    .list(&ListParams::default().labels(&format!("ame-task={}", task_name)))
                    .await
                    .unwrap();

                // TODO: ensure the pod is actually done before  giving up on logging.
                if task_pods.items.len() > 0 {
                    let pod = &task_pods.items[0];
                    debug!("found pod {} for task {}", &pod.name_any(), &task_name);

                    await_condition(
                        Api::<Pod>::clone(&pods),
                        &pod.name_any(),
                        conditions::is_pod_running(),
                    )
                    .await
                    .unwrap();

                    debug!("pod is running!");

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
                        .await
                        .unwrap();

                    while let Some(e) = pod_log_stream.next().await {
                        debug!("sent log entry: {:?}", &e);
                        log_sender
                            .send(Ok(LogEntry {
                                contents: e.unwrap().to_vec(),
                            }))
                            .await
                            .unwrap();
                    }

                    break;
                }

                let Ok(task) = tasks
                    .get_status(&task_name)
                    .await else {
                        sleep(Duration::from_secs(1)).await;
                        continue;
                    };

                log_sender
                    .send(Ok(LogEntry {
                        contents: format!("status: {} {:?}", &task_name, task.status.clone())
                            .as_bytes()
                            .to_vec(),
                    }))
                    .await
                    .unwrap();

                if task.status.clone().unwrap().phase.unwrap() != TaskPhase::Running
                    && task.status.unwrap().phase.unwrap() != TaskPhase::Pending
                {
                    break;
                }

                sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(log_reciever)))
    }
}

async fn build_server(cfg: AmeServiceConfig) -> Result<AService> {
    let client = Client::try_default().await.expect("Failed to create a K8S client, is the controller running in an environment with access to cluster credentials?");
    let target_namespace = "ame-system";
    let tasks = Api::<controller::Task>::namespaced(client.clone(), target_namespace);
    let pods = Api::<Pod>::namespaced(client.clone(), target_namespace);

    let task_service = AService {
        tasks: Arc::new(tasks),
        pods: Arc::new(pods),
        storage: Arc::new(ObjectStorage::<S3StorageDriver>::new_s3_storage(
            &cfg.bucket,
            cfg.s3config,
        )?),
    };

    Ok(task_service)
}

async fn health_check<S: NamedService>(
    mut reporter: HealthReporter,
    bucket: String,
    s3config: S3Config,
) {
    let client = Client::try_default().await.expect("Failed to create a K8S client, is the controller running in an environment with access to cluster credentials?");
    let tasks = Api::<controller::Task>::namespaced(client, "default");

    let storage = ObjectStorage::new_s3_storage(&bucket, s3config)
        .expect("failed to create object storage client");

    loop {
        let storage_health = storage.health_check().await;
        if storage_health.is_err() {
            reporter.set_not_serving::<S>().await;
            continue;
        } else {
            reporter.set_serving::<S>().await;
        }

        if (tasks.list(&ListParams::default()).await).is_ok() {
            reporter.set_serving::<S>().await;
        } else {
            reporter.set_not_serving::<S>().await;
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let logger = tracing_subscriber::fmt::layer();
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    let collector = Registry::default().with(logger).with(env_filter);
    tracing::subscriber::set_global_default(collector).unwrap();

    let s3config = S3Config::init_from_env().unwrap();
    let bucket = "ame".to_string();

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    tokio::spawn(health_check::<AmeServiceServer<AService>>(
        health_reporter.clone(),
        bucket.clone(),
        s3config.clone(),
    ));

    let svc = build_server(AmeServiceConfig { s3config, bucket }).await?;
    let addr = "0.0.0.0:3342".parse().unwrap();

    match svc.storage.ensure_storage_is_ready().await {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("failed to prepare object storage: {}", e);
            return Err(e);
        }
    };

    let classifier = GrpcErrorsAsFailures::new()
        .with_success(GrpcCode::InvalidArgument)
        .with_success(GrpcCode::NotFound);

    // Build our middleware stack
    let layer = ServiceBuilder::new()
        .timeout(Duration::from_secs(10))
        .layer(CompressionLayer::new())
        .layer(SetSensitiveHeadersLayer::new(once(header::AUTHORIZATION)))
        .layer(
            TraceLayer::new(SharedClassifier::new(classifier))
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_request(DefaultOnRequest::new().level(Level::INFO)),
        )
        .into_inner();

    tracing::event!(Level::INFO, "Serving at : {}", addr);

    Server::builder()
        .layer(
            TraceLayer::new_for_grpc()
                .on_request(tower_http::trace::DefaultOnRequest::new().level(Level::INFO)),
        )
        .accept_http1(true)
        .add_service(tonic_web::enable(AmeServiceServer::new(svc)))
        .add_service(health_service)
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use common::find_service_endpoint;
    use kube::api::PostParams;
    use kube::ResourceExt;
    use tonic::transport::Channel;

    use super::*;
    use ameservice::ame_service_client::AmeServiceClient;
    use serial_test::serial;

    async fn start_server() -> Result<AmeServiceClient<Channel>> {
        let port = "0.0.0.0:3342";
        let addr = port.parse().unwrap();

        let s3config = S3Config {
            region: "eu-central-1".to_string(),
            endpoint: find_service_endpoint("ame-system", "ame-minio")
                .await
                .unwrap(),
            access_id: "minio".to_string(),
            secret: "minio123".to_string(),
        };

        let svc = build_server(AmeServiceConfig {
            s3config,
            bucket: "ame".to_string(),
        })
        .await?;
        tokio::spawn(
            Server::builder()
                .add_service(AmeServiceServer::new(svc))
                .serve(addr),
        );

        // The server needs time to start serving requests.
        for _ in 0..2 {
            if let Ok(client) = AmeServiceClient::connect("http://".to_string() + port).await {
                return Ok(client);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        panic!("failed to start AME's server");
    }

    #[tokio::test]
    #[serial]
    async fn can_get_task() -> Result<()> {
        let mut service_client = start_server().await?;
        let client = Client::try_default().await?;
        let tasks = Api::<controller::Task>::default_namespaced(client);

        let task = TaskTemplate {
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            ..TaskTemplate::default()
        };

        let task_in_cluster = tasks
            .create(
                &PostParams::default(),
                &controller::Task::from(task.clone()),
            )
            .await?;

        let task_identifier = TaskIdentifier {
            name: task_in_cluster.name_any(),
        };

        let new_task = service_client
            .get_task(Request::new(task_identifier))
            .await?;

        assert_eq!(new_task.get_ref(), &task);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn can_create_task() -> Result<()> {
        let mut service_client = start_server().await?;
        let client = Client::try_default().await?;
        let tasks = Api::<controller::Task>::default_namespaced(client);

        let task = TaskTemplate {
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            ..TaskTemplate::default()
        };

        let create_task_req = CreateTaskRequest {
            id: Some(TaskIdentifier {
                name: "mytask".to_string(),
            }),
            templat: Some(task.clone()),
        };

        let new_task = service_client
            .create_task(Request::new(create_task_req.clone()))
            .await?;
        let task_in_cluster = tasks.get(&new_task.get_ref().name).await?;

        assert_eq!(TaskTemplate::from(task_in_cluster), task);

        service_client
            .delete_task(Request::new(create_task_req.id.unwrap()))
            .await?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn can_create_task_with_image_override() -> Result<()> {
        let mut service_client = start_server().await?;
        let client = Client::try_default().await?;
        let tasks = Api::<controller::Task>::default_namespaced(client);

        let task = TaskTemplate {
            name: "template_name".to_string(),
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            image: Some("my-new-image".to_string()),
        };

        let create_task_req = CreateTaskRequest {
            id: Some(TaskIdentifier {
                name: "mytask2".to_string(),
            }),
            templat: Some(task.clone()),
        };

        let new_task = service_client
            .create_task(Request::new(create_task_req.clone()))
            .await?;

        let task_in_cluster = tasks.get(&new_task.get_ref().name).await?;

        assert_eq!(TaskTemplate::from(task_in_cluster), task);

        service_client
            .delete_task(Request::new(create_task_req.id.unwrap()))
            .await?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn can_delete_task() -> Result<()> {
        let mut service_client = start_server().await?;
        let client = Client::try_default().await?;
        let tasks = Api::<controller::Task>::default_namespaced(client);

        let task_in_cluster = tasks
            .create(
                &PostParams::default(),
                &controller::Task::from(TaskTemplate {
                    command: "test".to_string(),
                    projectid: "myproject".to_string(),
                    ..TaskTemplate::default()
                }),
            )
            .await?;

        service_client
            .delete_task(Request::new(TaskIdentifier::from(task_in_cluster.clone())))
            .await?;

        let res = tasks.get(&task_in_cluster.name_any()).await.err().unwrap();

        assert!(
            matches!(
                &res,
                kube::Error::Api(e) if e.code == 404
            ),
            "failed to match code: {}",
            res
        );

        Ok(())
    }
}
