use crate::Result;
use ame::grpc::{TaskIdentifier, TaskProjectDirectoryStructure};
use async_trait::async_trait;
use envconfig::Envconfig;
use s3::{bucket::Bucket, creds::Credentials, BucketConfiguration, Region};

#[derive(PartialEq, Clone, Debug, Default)]
pub struct AmeFile {
    pub key: String,
    pub contents: Vec<u8>,
}

#[async_trait]
pub trait ObjectStorageDriver {
    async fn write(&self, prefix: &str, file: AmeFile) -> Result<()>;
    async fn read(&self, key: String) -> Result<AmeFile>;
    async fn list(&self, key: &str) -> Result<Vec<String>>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn init_storage(&self) -> Result<()>;
    async fn clear_storage(&self) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct ObjectStorage<T: ObjectStorageDriver> {
    pub driver: T,
}

#[derive(Clone, Debug, Envconfig)]
pub struct S3Config {
    #[envconfig(from = "S3_REGION", default = "eu-central-1")]
    pub region: String,

    #[envconfig(from = "S3_ENDPOINT", default = "http://ame-minio:9000")]
    pub endpoint: String,

    #[envconfig(from = "S3_ACCESS_ID")]
    pub access_id: String,

    #[envconfig(from = "S3_SECRET")]
    pub secret: String,
}

impl ObjectStorage<S3StorageDriver> {
    pub fn new_s3_storage(
        bucket_name: &str,
        config: S3Config,
    ) -> Result<ObjectStorage<S3StorageDriver>> {
        let driver = S3StorageDriver::new_init_bucket(bucket_name, config)?;
        Ok(ObjectStorage { driver })
    }
}

fn task_project_files_path(task_id: &TaskIdentifier) -> String {
    format!("tasks/{}/projectfiles", task_id.name)
}

fn task_project_file_path(task_id: &TaskIdentifier, project_path: &str) -> String {
    format!("tasks/{}/projectfiles/{}", task_id.name, project_path)
}

fn task_directory(task_id: &TaskIdentifier) -> String {
    format!("tasks/{}", task_id.name)
}

fn task_project_directory_structure_path(task_id: &TaskIdentifier) -> String {
    format!("{}/directory_structure", task_directory(task_id))
}

impl<T: ObjectStorageDriver> ObjectStorage<T> {
    pub fn new(driver: T) -> Self {
        ObjectStorage { driver }
    }

    pub async fn write_task_project_file(
        &self,
        task_id: &TaskIdentifier,
        file: AmeFile,
    ) -> Result<()> {
        self.driver
            .write(&task_project_files_path(task_id), file)
            .await
    }

    pub async fn list_task_project_files(&self, task_id: &TaskIdentifier) -> Result<Vec<String>> {
        self.driver.list(&task_project_files_path(task_id)).await
    }

    pub async fn ensure_storage_is_ready(&self) -> Result<()> {
        self.driver.init_storage().await
    }

    pub async fn store_project_directory_structure(
        &self,
        task_id: &TaskIdentifier,
        project_dir_struct: TaskProjectDirectoryStructure,
    ) -> Result<()> {
        let f = AmeFile {
            key: "directory_structure".to_string(),
            contents: serde_json::to_vec(&project_dir_struct)?,
        };

        self.driver.write(&task_directory(task_id), f).await
    }

    pub async fn get_task_project_directory_structure(
        &self,
        task_id: &TaskIdentifier,
    ) -> Result<TaskProjectDirectoryStructure> {
        let contents = self
            .driver
            .read(task_project_directory_structure_path(task_id))
            .await?;

        Ok(serde_json::from_slice(&contents.contents)?)
    }

    pub async fn get_task_project_file(
        &self,
        task_id: &TaskIdentifier,
        key: &str,
    ) -> Result<AmeFile> {
        self.driver.read(task_project_file_path(task_id, key)).await
    }

    pub async fn delete_task_project_file(
        &self,
        task_id: &TaskIdentifier,
        file_name: &str,
    ) -> Result<()> {
        self.driver
            .delete(&task_project_file_path(task_id, file_name))
            .await
    }

    pub async fn health_check(&self) -> Result<()> {
        self.driver.list("").await.map(|_| ())
    }
}

#[derive(Clone, Debug)]
pub struct S3StorageDriver {
    bucket: Bucket,
}

impl S3StorageDriver {
    fn new_init_bucket(bucket_name: &str, config: S3Config) -> Result<Self> {
        let bucket = Bucket::new(
            bucket_name,
            Region::Custom {
                region: config.region,
                endpoint: config.endpoint.to_string(),
            },
            Credentials::new(
                Some(&config.access_id),
                Some(&config.secret),
                None,
                None,
                None,
            )?,
        )?
        .with_path_style();

        Ok(S3StorageDriver { bucket })
    }

    async fn ensure_bucket_exists(&self) -> Result<()> {
        match self.bucket.list("".to_string(), None).await {
            Err(_) => {
                s3::bucket::Bucket::create_with_path_style(
                    &self.bucket.name,
                    self.bucket.region.clone(),
                    self.bucket.credentials.clone(),
                    BucketConfiguration::default(),
                )
                .await?;
                Ok(())
            }

            Ok(_) => Ok(()),
        }
    }
}

#[async_trait]
impl ObjectStorageDriver for S3StorageDriver {
    async fn write(&self, prefix: &str, file: AmeFile) -> Result<()> {
        self.bucket
            .put_object(format!("{}/{}", prefix, file.key), file.contents.as_slice())
            .await?;
        Ok(())
    }

    async fn read(&self, key: String) -> Result<AmeFile> {
        let obj = self.bucket.get_object(&key).await?;

        Ok(AmeFile {
            key: std::path::PathBuf::from(key)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            contents: obj.bytes().to_vec(),
        })
    }

    async fn list(&self, key: &str) -> Result<Vec<String>> {
        Ok(self
            .bucket
            .list(key.to_string(), None)
            .await?
            .into_iter()
            .flat_map(|r| r.contents.into_iter().map(|m| m.key))
            .collect())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.bucket.delete_object(key).await?;
        Ok(())
    }

    async fn init_storage(&self) -> Result<()> {
        Ok(self.ensure_bucket_exists().await?)
    }

    async fn clear_storage(&self) -> Result<()> {
        for e in self.list("").await? {
            self.delete(&e).await?;
        }

        self.bucket.delete().await?;
        Ok(())
    }
}
