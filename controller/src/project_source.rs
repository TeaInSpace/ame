use crate::project::{Project, ProjectSpec};
use crate::secrets::SecretCtrl;
use crate::{Error, Result};
use ame::grpc::GitProjectSource;
use ame::grpc::ProjectSourceState;
use ame::grpc::ProjectSourceStatus;
use duration_string::DurationString;
use envconfig::Envconfig;
use futures::{future::BoxFuture, FutureExt, StreamExt};
use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions};
use k8s_openapi::api::core::v1::Secret;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use k8s_openapi::chrono::DateTime;

use kube::{
    api::{Api, ListParams, PatchParams, ResourceExt},
    client::Client,
    core::ObjectMeta,
    runtime::controller::{Action, Controller},
    CustomResource, Resource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;
use std::{fs, sync::Arc, time::Duration};
use tracing::{info, warn};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "ProjectSource",
    group = "ame.teainspace.com",
    version = "v1alpha1",
    namespaced,
    status = "ProjectSourceStatus",
    shortname = "psrc"
)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSourceSpec {
    #[serde(flatten)]
    pub cfg: ame::grpc::ProjectSourceCfg,
}

#[derive(Envconfig, Clone)]
pub struct ProjectSrcCtrlCfg {
    #[envconfig(from = "NAMESPACE", default = "ame-system")]
    pub namespace: String,
}

impl ProjectSource {
    async fn git_secret(&self, secrets: Api<Secret>) -> Result<Option<String>> {
        let Some(GitProjectSource {  secret: Some(secret_name), .. }) = self.spec.clone().cfg.git else {
        return Ok(None);
    };
        Ok(Some(
            SecretCtrl::from(secrets).get_secret(&secret_name).await?,
        ))
    }

    async fn extract_projects(&self, secrets: Api<Secret>) -> Result<Vec<ProjectSpec>> {
        let Some(GitProjectSource{
        repository,
        username,
        ..
        }) = self.spec.clone().cfg.git else {
            return Err(Error::MissingProjectSrc("git".to_string()))
        };

        let _ = fs::remove_dir_all("/tmp/".to_string() + &self.name_any());

        let git_secret = self.git_secret(secrets).await?;

        let mut opts = FetchOptions::new();

        if git_secret.is_some() {
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.credentials(|_user, _, _| -> std::result::Result<Cred, git2::Error> {
                let Some(git_secret) = git_secret.clone() else {
                    return Err(git2::Error::from_str("git secret was empty"))
                };

                let Some(username) = username.clone() else {
                return Err(git2::Error::from_str("missing username"));

                };

                git2::Cred::userpass_plaintext(&username, &git_secret)
            });

            opts.remote_callbacks(callbacks);
        }

        let mut builder = RepoBuilder::new();
        builder.fetch_options(opts);

        // TODO: ensure that cloning never clashes with other directories.
        // TODO: How will we handle large repositories?
        let _repo = builder.clone(
            &repository,
            Path::new(&format!("/tmp/{}", &self.name_any())),
        )?;

        let project = serde_yaml::from_str(&fs::read_to_string(format!(
            "/tmp/{}/ame.yaml",
            self.name_any()
        ))?)?;

        fs::remove_dir_all("/tmp/".to_string() + &self.name_any())?;

        Ok(vec![project])
    }

    fn sync_interval(&self) -> Result<Duration> {
        if let Some(GitProjectSource {
            sync_interval: Some(sync_internal),
            ..
        }) = &self.spec.cfg.git
        {
            Ok(DurationString::try_from(sync_internal.clone())
                .map_err(Error::InvalidDuration)?
                .into())
        } else {
            Ok(Duration::from_secs(5 * 60))
        }
    }

    fn requires_sync(&self) -> Result<bool> {
        if let Some(ProjectSourceStatus {
            last_synced: Some(ref last_synced),
            ..
        }) = self.status
        {
            Ok(std::time::SystemTime::now()
                .duration_since(SystemTime::from(DateTime::parse_from_rfc2822(last_synced)?))?
                > self.sync_interval()?)
        } else {
            Ok(true)
        }
    }
}

struct Context {
    client: Client,
    config: ProjectSrcCtrlCfg,
}

async fn reconcile(src: Arc<ProjectSource>, ctx: Arc<Context>) -> Result<Action> {
    info!("Reconciling {}", src.name_any());

    let _srcs = Api::<ProjectSource>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let projects = Api::<Project>::namespaced(ctx.client.clone(), &ctx.config.namespace);
    let oref = if let Some(refe) = src.controller_owner_ref(&()) {
        refe
    } else {
        OwnerReference::default()
    };

    let secrets = Api::<Secret>::namespaced(ctx.client.clone(), &ctx.config.namespace);

    let mut patch: ProjectSource = _srcs.get_status(&src.name_any()).await?;
    patch.metadata.managed_fields = None;

    if src.requires_sync()? {
        let project_specs = match src.extract_projects(secrets).await {
            Ok(specs) => specs,
            Err(e) => {
                let status = ProjectSourceStatus {
                    state: ProjectSourceState::Error.into(),
                    reason: Some(e.to_string()),
                    ..patch.status.unwrap_or(ProjectSourceStatus::default())
                };

                patch.status = Some(status);

                _srcs
                    .patch_status(
                        &src.name_any(),
                        &PatchParams::apply("ame-controller"),
                        &kube::api::Patch::Apply(patch),
                    )
                    .await?;
                return Ok(Action::requeue(Duration::from_secs(50)));
            }
        };

        if project_specs.is_empty() {
            return Ok(Action::requeue(Duration::from_secs(50)));
        }

        let mut project = Project {
            metadata: ObjectMeta {
                name: Some(src.name_any()),
                ..ObjectMeta::default()
            },
            spec: project_specs[0].clone(),
            status: None,
        };

        if let Some(GitProjectSource { ref repository, .. }) = src.spec.cfg.git {
            project.add_annotation("gitrepository".to_string(), repository.to_string());
        }

        let project = project.add_owner_reference(oref);

        projects
            .patch(
                &project.name_any(),
                &PatchParams::apply("ame-controller"),
                &kube::api::Patch::Apply(project),
            )
            .await?;
    }

    let last_synced = Some(humantime::format_rfc3339(SystemTime::now()).to_string());
    let mut patch: ProjectSource = _srcs.get_status(&src.name_any()).await?;
    patch.metadata.managed_fields = None;

    if let Some(mut status) = patch.clone().status {
        status.last_synced = last_synced;
        status.reason = Some("project has been synced".to_string());
        status.state = ProjectSourceState::Synchronized.into();
        patch.status = Some(status);
    } else {
        patch.status = Some(ProjectSourceStatus {
            last_synced,
            reason: Some("project has been synced".to_string()),
            state: ProjectSourceState::Synchronized.into(),
            ..ProjectSourceStatus::default()
        })
    }

    _srcs
        .patch_status(
            &src.name_any(),
            &PatchParams::apply("ame-controller"),
            &kube::api::Patch::Apply(patch),
        )
        .await?;

    Ok(Action::requeue(Duration::from_secs(5 * 60)))
}

fn error_policy(_src: Arc<ProjectSource>, error: &Error, _ctx: Arc<Context>) -> Action {
    warn!("failed to reconcile: {:?}", error);
    Action::requeue(Duration::from_secs(5 * 60))
}

pub async fn start_project_source_controller(config: ProjectSrcCtrlCfg) -> BoxFuture<'static, ()> {
    let client = Client::try_default().await.expect("Failed to create a K8S client, is the controller running in an environment with access to cluster credentials?");
    let context = Arc::new(Context {
        client: client.clone(),
        config,
    });

    let project_srcs = Api::<ProjectSource>::namespaced(client.clone(), &context.config.namespace);
    project_srcs
        .list(&ListParams::default())
        .await
        .expect("Is the CRD installed?");

    let projects = Api::<Project>::namespaced(client, &context.config.namespace);

    Controller::new(project_srcs, ListParams::default())
        .owns(projects, ListParams::default())
        .run(reconcile, error_policy, context)
        .filter_map(|x| async move { std::result::Result::ok(x) })
        .for_each(|_| futures::future::ready(()))
        .boxed()
}

#[cfg(test)]
mod test {

    use crate::common::private_repo_gh_pat;

    use super::*;
    use crate::secrets::SecretCtrl;
    use assert_fs::prelude::*;
    use futures::StreamExt;
    use futures::TryStreamExt;
    use k8s_openapi::api::core::v1::Secret;
    use kube::api::DeleteParams;

    use kube::{api::PostParams, core::WatchEvent};
    use serde_json::json;
    use serial_test::serial;

    fn test_project_src() -> Result<ProjectSource> {
        Ok(serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "ProjectSource",
            "metadata": { "name": "test" },
            "spec": {
                "git": {
                    "repository": "https://github.com/TeaInSpace/ame-test.git",
                },
            }
        }))?)
    }

    fn private_test_project_src() -> Result<ProjectSource> {
        Ok(serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "ProjectSource",
            "metadata": { "name": "test-private" },
            "spec": {
                "git": {
                    "repository": "https://github.com/TeaInSpace/ame-test-private.git",
                    "secret": "ghsecret",
                    "username": "jmintb",
                },
            }
        }))?)
    }

    #[tokio::test]
    #[serial]
    async fn can_extract_projects_from_public_git_repository() -> Result<()> {
        let test_dir = assert_fs::TempDir::new().unwrap();
        let working_dir = std::env::current_dir()?;

        std::env::set_current_dir(test_dir.path())?;

        let project_src: ProjectSource = serde_json::from_value(json!({
            "apiVersion": "ame.teainspace.com/v1alpha1",
            "kind": "ProjectSource",
            "metadata": { "name": "test" },
            "spec": {
                "git": {
                    "repository": "https://github.com/TeaInSpace/ame-test.git",
                },
            }
        }))?;

        let client = Client::try_default().await?;
        let secrets = Api::<Secret>::default_namespaced(client);
        let projects = project_src.extract_projects(secrets).await?;
        insta::assert_yaml_snapshot!(&projects);

        test_dir
            .child(project_src.name_any())
            .assert(predicates::path::missing());

        std::env::set_current_dir(working_dir)?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn can_create_project_from_src() -> Result<()> {
        let client = Client::try_default().await?;
        let project_srcs = Api::<ProjectSource>::default_namespaced(client.clone());
        let projects = Api::<Project>::default_namespaced(client.clone());

        let _handle = tokio::spawn(
            start_project_source_controller(ProjectSrcCtrlCfg {
                namespace: "default".to_string(),
            })
            .await,
        );

        let project_src = test_project_src()?;

        let _ = project_srcs
            .delete(&project_src.name_any(), &DeleteParams::default())
            .await;

        project_srcs
            .create(&PostParams::default(), &project_src)
            .await?;

        let mut stream = projects
            .watch(&ListParams::default().timeout(60), "0")
            .await?
            .boxed();
        while let Ok(Some(e)) = stream.try_next().await {
            if let WatchEvent::Added(_project) = e {
                return Ok(());
            }
        }

        panic!("failed to create project ",);
    }

    #[tokio::test]
    #[serial]
    async fn can_create_project_from_private_src() -> Result<()> {
        let client = Client::try_default().await?;
        let project_srcs = Api::<ProjectSource>::default_namespaced(client.clone());
        let projects = Api::<Project>::default_namespaced(client.clone());

        let secret_ctrl = SecretCtrl::new(client, "default");

        let secret_name = "ghsecret";
        secret_ctrl
            .store_secret_if_empty(secret_name, private_repo_gh_pat().unwrap())
            .await
            .unwrap();

        let _handle = tokio::spawn(
            start_project_source_controller(ProjectSrcCtrlCfg {
                namespace: "default".to_string(),
            })
            .await,
        );

        let mut project_src = private_test_project_src()?;

        let _ = project_srcs
            .delete(&project_src.name_any(), &DeleteParams::default())
            .await;

        project_src = project_srcs
            .create(&PostParams::default(), &project_src)
            .await?;

        let mut stream = projects
            .watch(&ListParams::default().timeout(60), "0")
            .await?
            .boxed();
        while let Ok(Some(e)) = stream.try_next().await {
            if let WatchEvent::Added(project) = e {
                if project
                    .metadata
                    .owner_references
                    .unwrap()
                    .iter()
                    .any(|r| r.name == project_src.name_any())
                {
                    return Ok(());
                }
            }
        }

        panic!("failed to create project ",);
    }

    //TODO test that sync interval can be overridden
    //TODO test that sync works
}
