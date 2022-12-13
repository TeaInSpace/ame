use std::time::Duration;

use controller::{Task, Workflow};
use either::Either;
use k8s_openapi::api::core::v1::{LoadBalancerStatus, Service, ServiceSpec, ServiceStatus};
use kube::{
    api::{DeleteParams, ListParams},
    Api, Client,
};

pub async fn find_service_endpoint(
    namespace: &str,
    service_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let services = Api::<Service>::namespaced(client.clone(), namespace);
    let service = services.get(service_name).await?;

    let Service { spec: Some(ServiceSpec{
       ports: Some(ports),
       ..
    }), status: Some(ServiceStatus{

        load_balancer: Some(LoadBalancerStatus{
            ingress: Some(ingress)
        }),
        ..
    }),
    ..} = service else {
        return Err(format!("failed to extract service ips and ports: {service:#?}"))?; 
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

    Ok(format!(
        "http://{}:{}",
        ingress[0].ip.as_ref().unwrap(),
        ports[0].port
    ))
}

/// Prepare a cluster for tests, under the assumptions that the `just setup_cluster` recipe has been run successfully.
/// This implies that all required custom resource definitions are installed in the cluster.
/// This function will generate clients and clear all Task and `Workflow` objects in the cluster.
pub async fn setup_cluster(
    namespace: &str,
) -> Result<(Api<controller::Task>, Api<controller::Workflow>), Box<dyn std::error::Error>> {
    let client = Client::try_default().await?;
    let tasks = Api::<Task>::namespaced(client.clone(), namespace);
    let workflows = Api::<Workflow>::namespaced(client.clone(), namespace);

    let dp = DeleteParams::default();
    let lp = ListParams::default();

    match tasks.delete_collection(&dp, &lp).await? {
        Either::Left(_) => {
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
