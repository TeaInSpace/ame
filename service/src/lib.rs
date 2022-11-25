pub mod ameservice;
pub use ameservice::*;
use s3::{creds::error::CredentialsError, error::S3Error};
use thiserror::Error;

mod storage;
use storage::{ObjectStorage, S3Config, S3StorageDriver};

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
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::storage::{AmeFile, ObjectStorageDriver};

    use super::*;
    use common::find_service_endpoint;
    use serial_test::serial;
    use tokio;

    async fn get_fresh_storage() -> Result<ObjectStorage<S3StorageDriver>> {
        let s3config = S3Config {
            region: "eu-central-1".to_string(),
            endpoint: find_service_endpoint("ame-system", "ame-minio")
                .await
                .unwrap(),
            access_id: "minio".to_string(),
            secret: "minio123".to_string(),
        };

        println!("service enpoint: {}", s3config.endpoint);

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
