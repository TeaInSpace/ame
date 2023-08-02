use ame::grpc::ame_service_server::AmeServiceServer;
use envconfig::Envconfig;
use service::{
    ameservice::{AmeServiceConfig, Service},
    health_check,
    storage::S3Config,
    Result,
};
use tonic::transport::Server;

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

    let websvc = tonic_web::config()
        .allow_all_origins()
        .enable(AmeServiceServer::new(svc));

    Server::builder()
        .accept_http1(true)
        .add_service(websvc)
        .add_service(health_service)
        .serve(addr)
        .await?;

    Ok(())
}
