pub mod ameservice;
pub mod storage;

use ame::custom_resources::task::Task;
use kube::api::ListParams;
use kube::runtime::wait;
use kube::{Api, Client};
use s3::{creds::error::CredentialsError, error::S3Error};
use std::net::AddrParseError;
use std::string::FromUtf8Error;
use std::time::Duration;
use storage::S3Config;
use thiserror::Error;

use tonic::transport::NamedService;
use tonic_health::server::HealthReporter;

use storage::ObjectStorage;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Got error from gRPC: {0}")]
    TonicError(#[from] tonic::transport::Error),

    #[error("Got error from gRPC: {0}")]
    TonicStatus(#[from] tonic::Status),

    //TODO: can we have from and source at the same time?
    #[error("Failed to create workflow: {0}")]
    KubeApiError(#[from] kube::Error),

    #[error("Got an object storage error: {0}")]
    S3Error(#[from] S3Error),

    #[error("Got a credential error from object storage: {0}")]
    S3CredentialError(#[from] CredentialsError),

    #[error("Got a serde JSON error: {0}")]
    SerdeJSONError(#[from] serde_json::Error),

    #[error("got error while converting: {0}")]
    ConversionError(String),

    #[error("got error while parsing address: {0}")]
    AddrParseError(#[from] AddrParseError),

    #[error("Got error from kube-rs runtime: {0}")]
    KubeRuntime(#[from] wait::Error),

    #[error("Got error from tokio send: {0}")]
    TokioSendError(String),

    #[error("Got error from formatting: {0}")]
    FormatError(#[from] FromUtf8Error),

    #[error("Failed to find the requested model: {0}")]
    MissingModel(String),

    #[error("Invalid project source: {0}")]
    InvalidProjectSrc(String),

    #[error("Failed to find project source for repository: {0}")]
    MissingProjectSrc(String),
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub async fn health_check<S: NamedService>(
    mut reporter: HealthReporter,
    bucket: String,
    s3config: S3Config,
) -> Result<()> {
    let client = Client::try_default().await.expect("Failed to create a K8S client, is the controller running in an environment with access to cluster credentials?");
    let tasks = Api::<Task>::namespaced(client, "default");

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

#[cfg(test)]
mod test {
    use super::storage::{AmeFile, ObjectStorage, ObjectStorageDriver, S3Config, S3StorageDriver};
    use super::Result;
    use ame::custom_resources::common::find_service_endpoint;
    use ame::grpc::{TaskIdentifier, TaskProjectDirectoryStructure};
    use serial_test::serial;

    async fn get_fresh_storage() -> Result<ObjectStorage<S3StorageDriver>> {
        let s3config = S3Config {
            region: "eu-central-1".to_string(),
            endpoint: find_service_endpoint("ame-system", "ame-minio")
                .await
                .unwrap(),
            access_id: "minio".to_string(),
            secret: "minio123".to_string(),
        };

        println!("service endpoint: {}", s3config.endpoint);

        let object_storage = ObjectStorage::new_s3_storage("test", s3config)?;

        object_storage.ensure_storage_is_ready().await?;

        Ok(object_storage)
    }

    #[tokio::test]
    #[serial]
    async fn s3_crud_operations() -> Result<()> {
        let object_storage = get_fresh_storage().await?;

        assert_eq!(object_storage.driver.list("").await?.len(), 0);

        let task_id = TaskIdentifier {
            name: "mytask".to_string(),
        };

        let file = AmeFile {
            key: "somekey".to_string(),
            contents: "somecontents".as_bytes().to_vec(),
        };

        object_storage
            .write_task_project_file(&task_id, file.clone())
            .await?;

        let stored_file = object_storage
            .get_task_project_file(&task_id, &file.key)
            .await?;

        assert_eq!(
            object_storage
                .list_task_project_files(&task_id)
                .await?
                .len(),
            1
        );

        object_storage
            .delete_task_project_file(&task_id, &file.key)
            .await?;

        assert_eq!(
            object_storage
                .list_task_project_files(&task_id)
                .await?
                .len(),
            0
        );

        assert_eq!(file, stored_file);

        object_storage.driver.clear_storage().await?;

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn can_storage_project_directory_structure() -> Result<()> {
        let object_storage = get_fresh_storage().await?;

        let task_id = TaskIdentifier {
            name: "mytask".to_string(),
        };

        let dir_structure = TaskProjectDirectoryStructure {
            projectid: "my_project".to_string(),
            taskid: Some(task_id.clone()),
            paths: vec!["models".to_string(), "data/val".to_string()],
        };

        assert_eq!(
            object_storage
                .list_task_project_files(&task_id)
                .await?
                .len(),
            0
        );

        object_storage
            .store_project_directory_structure(&task_id, dir_structure.clone())
            .await?;

        let stored_structure = object_storage
            .get_task_project_directory_structure(&task_id)
            .await?;

        assert_eq!(dir_structure, stored_structure);

        object_storage.driver.clear_storage().await?;

        Ok(())
    }
}
