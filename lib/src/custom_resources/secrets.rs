use crate::grpc::AmeSecretId;
use k8s_openapi::{api::core::v1::Secret, ByteString};
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    core::ObjectMeta,
    error::ErrorResponse,
    Api, Client, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, string::FromUtf8Error};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SecretError {
    #[error("error communicating with Kubernetes: {0}")]
    KubeApi(#[from] kube::Error),

    #[error("secret with key: {0} was not found")]
    MissingSecret(String),

    #[error("secret with key: {0}, was misconfigured")]
    MissingSecretKey(String),

    #[error("{0}")]
    FailedToParseString(#[from] FromUtf8Error),
}

impl From<SecretError> for tonic::Status {
    fn from(error: SecretError) -> Self {
        match &error {
            SecretError::MissingSecret(_key) => Self::new(tonic::Code::NotFound, error.to_string()),
            _ => Self::from_error(Box::new(error)),
        }
    }
}

type Result<T> = std::result::Result<T, SecretError>;

pub type AmeSecretVal = String;

static AME_SECRET_STORE: &str = "ame";

#[derive(Debug)]
pub struct SecretCtrl {
    secrets: Api<Secret>,
}

pub trait ResourceBuilder {
    fn label(&mut self, key: String, val: String) -> &mut Self;
}

impl From<Api<Secret>> for SecretCtrl {
    fn from(secrets: Api<Secret>) -> Self {
        Self { secrets }
    }
}
fn secrets_list_params() -> ListParams {
    ListParams::default().labels(&format!("SECRET_STORE={AME_SECRET_STORE}"))
}

fn is_ame_secret(secret: &Secret) -> bool {
    if let Some(val) = secret.labels().get("SECRET_STORE") {
        val == AME_SECRET_STORE
    } else {
        false
    }
}

impl SecretCtrl {
    pub fn new(client_cfg: Client, ns: &str) -> Self {
        Self {
            secrets: Api::namespaced(client_cfg, ns),
        }
    }

    pub async fn try_default(ns: &str) -> Result<Self> {
        let client_cfg = Client::try_default().await?;

        Ok(Self {
            secrets: Api::namespaced(client_cfg, ns),
        })
    }

    pub async fn store_secret(&self, key: &str, val: String) -> Result<()> {
        let mut secret_map = BTreeMap::new();
        secret_map.insert("secret".to_string(), val);

        let mut labels = BTreeMap::new();
        labels.insert("SECRET_STORE".to_string(), AME_SECRET_STORE.to_string());

        let secret = Secret {
            metadata: ObjectMeta {
                name: Some(key.to_string()),
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            string_data: Some(secret_map),
            ..Secret::default()
        };

        self.secrets.create(&PostParams::default(), &secret).await?;

        Ok(())
    }

    pub async fn store_secret_if_empty(&self, key: &str, val: String) -> Result<()> {
        let res = self.store_secret(key, val).await;
        match res {
            Err(SecretError::KubeApi(kube::Error::Api(ErrorResponse { code: 409, .. })))
            | Ok(_) => Ok(()), // If the K8S API returns a conflice (409) the secret already exists.
            Err(e) => Err(e)?,
        }
    }

    pub async fn delete_secret(&self, key: &str) -> Result<()> {
        self.get_secret(key).await?;
        self.secrets.delete(key, &DeleteParams::default()).await?;
        Ok(())
    }

    pub async fn get_secret(&self, key: &str) -> Result<AmeSecretVal> {
        let secret = self.secrets.get(key).await?;

        if !is_ame_secret(&secret) {
            return Err(SecretError::MissingSecret(key.to_string()));
        }

        let Some(secret_data) = secret.data else {
            return Err(SecretError::MissingSecret(key.to_string()));
        };

        if let Some(ByteString(v)) = secret_data.get("secret").to_owned() {
            Ok(String::from_utf8(v.to_owned())?)
        } else {
            Err(SecretError::MissingSecretKey(key.to_string()))
        }
    }

    pub async fn list_secrets(&self) -> Result<Vec<AmeSecretId>> {
        Ok(self
            .secrets
            .list(&secrets_list_params())
            .await?
            .into_iter()
            .map(|s| s.name_any().into())
            .collect())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum SecretReference {
    AmeSecret(String),
}

#[cfg(test)]
mod test {
    use super::{Result, SecretCtrl};
    use insta::assert_yaml_snapshot;
    use k8s_openapi::api::core::v1::Secret;
    use kube::{Api, Client, ResourceExt};

    #[tokio::test]
    #[ignore]
    async fn can_label_secrets() -> Result<()> {
        let client_cfg = Client::try_default().await?;
        let secrets = Api::<Secret>::default_namespaced(client_cfg);
        let secret_ctrl = SecretCtrl::from(secrets.clone());
        let secret_key = "mytestsecret";
        let _ = secret_ctrl.delete_secret(secret_key).await;
        secret_ctrl
            .store_secret(secret_key, "testval".to_string())
            .await?;

        let secret = secrets.get(secret_key).await?;
        assert_yaml_snapshot!(secret.labels());

        secret_ctrl.delete_secret(secret_key).await?;
        Ok(())
    }
}
