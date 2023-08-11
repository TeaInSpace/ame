use crate::{
    custom_resources::{argo::Workflow, data_set::DataSet, new_task::Task, project::Project},
    error::AmeError,
};
use either::Either;
use k8s_openapi::{
    api::core::v1::{LoadBalancerStatus, Service, ServiceSpec, ServiceStatus},
    apimachinery::pkg::apis::meta::v1::OwnerReference,
};
use kube::{
    api::{DeleteParams, ListParams, PatchParams},
    Api, Client, ResourceExt,
};

use std::time::Duration;

use super::project_source::ProjectSource;

pub async fn find_ame_endpoint(
    namespace: &str,
    service_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let services = Api::<Service>::namespaced(client.clone(), namespace);
    let service = services.get(service_name).await?;

    let Service {
        spec: Some(ServiceSpec {
            ports: Some(ports), ..
        }),
        ..
    } = service
    else {
        return Err(format!(
            "failed to extract service ips and ports: {service:#?}"
        ))?;
    };

    let port = ports
        .iter()
        .find(|p| p.name.clone().unwrap_or("".to_string()) == "https");

    if let Some(port) = port {
        Ok(format!("https://ame.local:{}", port.port))
    } else {
        Err("failed to find a port".to_string())?
    }
}

pub async fn find_service_endpoint(
    namespace: &str,
    service_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let services = Api::<Service>::namespaced(client.clone(), namespace);
    let service = services.get(service_name).await?;

    let Service {
        spec: Some(ServiceSpec {
            ports: Some(ports), ..
        }),
        status:
            Some(ServiceStatus {
                load_balancer:
                    Some(LoadBalancerStatus {
                        ingress: Some(ingress),
                    }),
                ..
            }),
        ..
    } = service
    else {
        return Err(format!(
            "failed to extract service ips and ports: {service:#?}"
        ))?;
    };

    if ingress.len() != 1 {
        return Err(format!(
            "expected a ingress but got {} for {} {:#?}",
            ingress.len(),
            service_name,
            ingress
        ))?;
    }

    if ports.len() != 1 {
        return Err(format!(
            "expected a port but got {} for {} {:#?}",
            ports.len(),
            service_name,
            ports
        ))?;
    }

    let Some(ip) = ingress[0].ip.as_ref() else {
        return Err("could not find IP from ingress".to_string().into());
    };

    Ok(format!("http://{}:{}", ip, ports[0].port))
}

pub fn parent_project(owner_references: Vec<OwnerReference>) -> crate::Result<String> {
    let projects: Vec<OwnerReference> = owner_references
        .into_iter()
        .filter(|o| kind_is_project(&o.kind))
        .collect();

    if projects.len() != 1 {
        return Err(AmeError::MissingProject(projects.len()));
    }

    Ok(projects[0].name.clone())
}

pub fn kind_is_project(kind: &str) -> bool {
    kind == "Project"
}

/// Prepare a cluster for tests, under the assumptions that the `just setup_cluster` recipe has been run successfully.
/// This implies that all required custom resource definitions are installed in the cluster.
/// This function will generate clients and clear all Task and `Workflow` objects in the cluster.
pub async fn setup_cluster(
    namespace: &str,
) -> Result<(Api<Task>, Api<Workflow>), Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let tasks = Api::<Task>::namespaced(client.clone(), namespace);
    let workflows = Api::<Workflow>::namespaced(client.clone(), namespace);
    let project_srcs = Api::<ProjectSource>::namespaced(client.clone(), namespace);
    let projects = Api::<Project>::namespaced(client.clone(), namespace);
    let data_sets = Api::<DataSet>::namespaced(client.clone(), namespace);

    let dp = DeleteParams::default();
    let lp = ListParams::default();
    let patch_params = PatchParams::apply("AME_TEST").force();

    for mut project in projects.list(&lp).await?.into_iter() {
        project.spec.deletion_approved = true;
        project.metadata.managed_fields = None;

        let res = projects
            .patch(
                &project.name_any(),
                &patch_params,
                &kube::api::Patch::Apply(project),
            )
            .await;

        match res {
            Err(kube::Error::Api(e)) => {
                if e.code == 409 {
                    return Ok((tasks, workflows));
                }
            }
            Err(e) => return Err(Box::new(e)),
            _ => (),
        }
    }

    match project_srcs.delete_collection(&dp, &lp).await? {
        Either::Left(_) => {
            while !project_srcs.list(&lp).await?.items.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Either::Right(status) => {
            println!("Deleted collection of tasks: {status:?}");
        }
    };
    for mut task in tasks.list(&lp).await?.into_iter() {
        task.spec.deletion_approved = true;
        task.metadata.managed_fields = None;

        tasks
            .patch(
                &task.name_any(),
                &patch_params,
                &kube::api::Patch::Apply(task),
            )
            .await?;
    }

    match projects.delete_collection(&dp, &lp).await? {
        Either::Left(_) => {
            for mut project in projects.list(&lp).await?.into_iter() {
                project.spec.deletion_approved = true;
                project.metadata.managed_fields = None;

                let _ = projects
                    .patch(
                        &project.name_any(),
                        &patch_params,
                        &kube::api::Patch::Apply(project),
                    )
                    .await;
            }
            while !projects.list(&lp).await?.items.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Either::Right(status) => {
            println!("Deleted collection of tasks: {status:?}");
        }
    };

    match data_sets.delete_collection(&dp, &lp).await? {
        Either::Left(_) => {
            for mut data_set in data_sets.list(&lp).await?.into_iter() {
                data_set.spec.deletion_approved = true;

                data_sets
                    .patch(
                        &data_set.name_any(),
                        &PatchParams::default(),
                        &kube::api::Patch::Merge(data_set),
                    )
                    .await?;
            }

            while !data_sets.list(&lp).await?.items.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Either::Right(status) => {
            println!("Deleted collection of tasks: {status:?}");
        }
    };

    match tasks.delete_collection(&dp, &lp).await? {
        Either::Left(_) => {
            for mut task in tasks.list(&lp).await?.into_iter() {
                task.spec.deletion_approved = true;
                task.metadata.managed_fields = None;

                tasks
                    .patch(
                        &task.name_any(),
                        &patch_params,
                        &kube::api::Patch::Apply(task),
                    )
                    .await?;
            }

            while !tasks.list(&lp).await?.items.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Either::Right(status) => {
            println!("Deleted collection of tasks: {status:?}");
        }
    };

    match workflows.delete_collection(&dp, &lp).await? {
        Either::Left(_) => {
            while !workflows.list(&lp).await?.items.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        Either::Right(status) => {
            println!("Deleted collection of tasks: {status:?}");
        }
    };

    Ok((tasks, workflows))
}

// This is an access token for https://github.com/TeaInSpace/ame-test-private intended for
// testing that AME can pull in projects from private repositories.
pub fn private_repo_gh_pat() -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::env::var("AME_TEST_GH_TOKEN")?)
}
