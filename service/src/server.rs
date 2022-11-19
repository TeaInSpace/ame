pub mod taskservice {
    tonic::include_proto!("taskservice");
}

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{DeleteParams, PostParams};
use kube::{Api, Client, ResourceExt};
use log::{debug, error, info, log_enabled, Level};
use std::sync::Arc;
use taskservice::task_service_server::{TaskService, TaskServiceServer};
use taskservice::{Empty, Task, TaskIdentifier};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Got error from gRPC: {0}")]
    TonicError(#[from] tonic::transport::Error),

    #[error("Got error from gRPC: {0}")]
    TonicStatus(#[from] tonic::Status),

    //TODO: can we have from and source at the same time?
    #[error("Failed to create workflow: {0}")]
    KubeApiError(#[from] kube::Error),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl From<Task> for controller::Task {
    fn from(t: Task) -> Self {
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

impl From<controller::Task> for Task {
    fn from(t: controller::Task) -> Self {
        Task {
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
struct TaskServiceConfig {
    tasks: Api<controller::Task>,
}

#[derive(Debug, Clone)]
struct TService {
    config: Arc<TaskServiceConfig>,
}

#[tonic::async_trait]
impl TaskService for TService {
    async fn get_task(&self, request: Request<TaskIdentifier>) -> Result<Response<Task>, Status> {
        let task = self
            .config
            .tasks
            .get(&request.get_ref().name)
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(Response::new(Task::from(task)))
    }

    async fn create_task(
        &self,
        request: Request<Task>,
    ) -> Result<Response<TaskIdentifier>, Status> {
        let task_in_cluster = self
            .config
            .tasks
            .create(
                &PostParams::default(),
                &controller::Task::from(request.into_inner()),
            )
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(Response::new(TaskIdentifier::from(task_in_cluster)))
    }

    async fn delete_task(
        &self,
        request: Request<TaskIdentifier>,
    ) -> Result<Response<Empty>, Status> {
        self.config
            .tasks
            .delete(&request.get_ref().name, &DeleteParams::default())
            .await
            .map_or_else(
                |e| Err(Status::from_error(Box::new(e))),
                |_| Ok(Response::new(Empty {})),
            )
    }
}

async fn build_server() -> Result<TService> {
    let client = Client::try_default().await.expect("Failed to create a K8S client, is the controller running in an environment with access to cluster credentials?");
    let tasks = Api::<controller::Task>::namespaced(client.clone(), "default");

    let task_service = TService {
        config: Arc::new(TaskServiceConfig { tasks }),
    };

    Ok(task_service)
}

fn request_logger(req: Request<()>) -> Result<Request<()>, Status> {
    info!("{:?}", req);
    println!("got req");
    Ok(req)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    let addr = "0.0.0.0:3342".parse().unwrap();

    let svc = build_server().await?;

    Server::builder()
        .add_service(health_service)
        .add_service(TaskServiceServer::with_interceptor(svc, request_logger))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use kube::api::PostParams;
    use kube::ResourceExt;
    use tonic::transport::Channel;

    use super::*;
    use serial_test::serial;
    use taskservice::task_service_client::TaskServiceClient;

    async fn start_server() -> Result<TaskServiceClient<Channel>> {
        let port = "[::1]:10000";
        let addr = port.parse().unwrap();

        let svc = build_server().await?;
        tokio::spawn(
            Server::builder()
                .add_service(TaskServiceServer::new(svc))
                .serve(addr),
        );

        // The server needs time to start serving requests.
        for _ in 0..2 {
            if let Ok(client) = TaskServiceClient::connect("http://".to_string() + port).await {
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

        let task = Task {
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            ..Task::default()
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

        let task = Task {
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            ..Task::default()
        };

        let new_task = service_client
            .create_task(Request::new(task.clone()))
            .await?;
        let task_in_cluster = tasks.get(&new_task.get_ref().name).await?;

        assert_eq!(Task::from(task_in_cluster), task);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn can_create_task_with_image_override() -> Result<()> {
        let mut service_client = start_server().await?;
        let client = Client::try_default().await?;
        let tasks = Api::<controller::Task>::default_namespaced(client);

        let task = Task {
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            image: Some("my-new-image".to_string()),
            ..Task::default()
        };

        let new_task = service_client
            .create_task(Request::new(task.clone()))
            .await?;

        let task_in_cluster = tasks.get(&new_task.get_ref().name).await?;

        assert_eq!(Task::from(task_in_cluster), task);

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
                &controller::Task::from(Task {
                    command: "test".to_string(),
                    projectid: "myproject".to_string(),
                    ..Task::default()
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
