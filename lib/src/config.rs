use std::env::var;

use crate::{error::AmeError, Result};
use k8s_openapi::api::{core::v1::ContainerImage, networking::v1::Ingress};
use serde::{Deserialize, Serialize};
use url::{Host, Url};

use crate::k8s_safe_types::ImagePullPolicy;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AmeCfg {
    server_port: u16,
    object_storage_endpoint: url::Url,
    object_storage_container: String,
    object_storage_secret_name: String,
    object_storage_secret_key: String,
    object_storage_id_key: String,
    mlflow_endpoint: Option<url::Url>,
    model_deployment_default_host: Option<Host>,
    model_deployment_ingress: Option<Ingress>,
    model_deployment_default_image_pull_policy: ImagePullPolicy,
    task_executor_default_image: String,
    task_executor_default_image_pull_policy: ImagePullPolicy,
}

impl AmeCfg {
    pub fn from_env() -> Result<Self> {
        let server_port: i32 = ame_env_var("SERVER_PORT")
            .map(|v| {
                v.parse()
                    .map_err(|e| AmeError::Parsing(format!("failed to parse server port {} ", v)))
            })
            .unwrap_or(Ok(3342))?;

        let object_storage_endpoint: Url = ame_env_var("OBJECT_STORAGE_ENDPOINT")
            .unwrap_or("http://ame-minio:9000".to_string())
            .parse()
            .map_err(|e| {
                AmeError::Parsing(format!(
                    "failed to pass object storage endpoint with error {e}"
                ))
            })?;

        // TODO: sanitize this
        let object_storage_endpoint: String =
            ame_env_var("OBJECT_STORAGE_CONTAINER").unwrap_or("ameprojectstorage".to_string());

        let object_storage_secret_name: String =
            ame_env_var("OBJECT_STORAGE_SECRET_NAME").unwrap_or("ame-minio".to_string());

        let object_storage_secret_key: String =
            ame_env_var("OBJECT_STORAGE_SECRET_KEY").unwrap_or("root-password".to_string());

        let object_storage_id_key: String =
            ame_env_var("OBJECT_STORAGE_ID_KEY").unwrap_or("root-user".to_string());

        let mlflow_endpoint: Option<Url> = ame_env_var("MLFLOW_ENDPOINT")
            .map(|v| {
                v.parse().map_or_else(
                    |e| {
                        Err(AmeError::Parsing(format!(
                            "failed to pass mlflow endpoint: {v} due to error {e}"
                        )))
                    },
                    |v| Ok(Some(v)),
                )
            })
            .unwrap_or(Ok(None))?;

        let model_deployment_default_host: Option<Host<String>> =
            ame_env_var("MODEL_DEPLOYMENT_DEFAULT_HOST")
                .map(|v| {
                    Host::parse(&v)
                        .map_or_else(|e| Err(AmeError::Parsing("".to_string())), |v| Ok(Some(v)))
                })
                .unwrap_or(Ok(None))?;

        todo!()
    }
}

fn ame_env_var(key: &str) -> Option<String> {
    var(format!("AME_{key}")).ok()
}
