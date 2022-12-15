use envconfig::Envconfig;
use service::ameservice::ame_service_server::AmeServiceServer;
use service::ameservice::AmeServiceConfig;
use service::ameservice::Service;
use service::health_check;
use service::storage::S3Config;
use service::Result;
use tonic::transport::Server;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

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
    tokio::spawn(health_check::<AmeServiceServer<Service>>(
        health_reporter.clone(),
        bucket.clone(),
        s3config.clone(),
    ));

    let svc = Service::try_init(AmeServiceConfig { s3config, bucket }).await?;
    svc.prepare_environment().await?;

    let addr = "0.0.0.0:3342".parse()?;
    tracing::info!("Serving at: {}", addr);

    Server::builder()
        .layer(TraceLayer::new_for_grpc())
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

    use common::{find_service_endpoint, setup_cluster};
    use kube::api::PostParams;
    use kube::ResourceExt;
    use kube::{Api, Client};

    use service::ameservice::{CreateTaskRequest, TaskIdentifier, TaskTemplate};
    use tokio::net::TcpListener;
    use tonic::transport::Channel;
    use tonic::Request;

    use super::AmeServiceConfig;
    use super::*;
    use serial_test::serial;
    use service::ameservice::ame_service_client::AmeServiceClient;
    use service::storage::S3Config;
    use service::Result;

    async fn start_server() -> Result<AmeServiceClient<Channel>> {
        setup_cluster("ame-system").await.unwrap();

        // Request a free port from the os.
        let addr = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap()
            .local_addr()
            .unwrap();

        let s3config = S3Config {
            region: "eu-central-1".to_string(),
            endpoint: find_service_endpoint("ame-system", "ame-minio")
                .await
                .unwrap(),
            access_id: "minio".to_string(),
            secret: "minio123".to_string(),
        };

        let svc = Service::try_init(AmeServiceConfig {
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
            if let Ok(client) = AmeServiceClient::connect(format!("http://{addr}")).await {
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
        let tasks = Api::<controller::Task>::namespaced(client, "ame-system");

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
        let tasks = Api::<controller::Task>::namespaced(client, "ame-system");

        let task = TaskTemplate {
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            ..TaskTemplate::default()
        };

        let create_task_req = CreateTaskRequest {
            id: Some(TaskIdentifier {
                name: "mytask".to_string(),
            }),
            template: Some(task.clone()),
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
        let tasks = Api::<controller::Task>::namespaced(client, "ame-system");

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
            template: Some(task.clone()),
        };

        let _new_task = service_client
            .create_task(Request::new(create_task_req.clone()))
            .await?;

        let task_in_cluster = tasks
            .get(&create_task_req.id.as_ref().unwrap().name)
            .await?;

        assert_eq!(
            task_in_cluster.spec.image,
            controller::Task::try_from(create_task_req.clone())?
                .spec
                .image
        );

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
        let tasks = Api::<controller::Task>::namespaced(client, "ame-system");

        let task = TaskTemplate {
            name: "template_name".to_string(),
            command: "test".to_string(),
            projectid: "myproject".to_string(),
            ..TaskTemplate::default()
        };

        let task_in_cluster = tasks
            .create(&PostParams::default(), &controller::Task::from(task))
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
            "failed to match code: {res}"
        );

        Ok(())
    }
}
