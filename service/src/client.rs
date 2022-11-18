pub mod taskservice {
    tonic::include_proto!("taskservice");
}

use taskservice::task_service_client::TaskServiceClient;
use taskservice::{Task, TaskIdentifier};
use tonic::{Request, Response, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TaskServiceClient::connect("http://localhost:3342").await?;

    let res = client
        .get_task(Request::new(TaskIdentifier {
            name: "trainingcm9sr".to_string(),
        }))
        .await?;

    println!("res: {:?}", res);
    Ok(())
}
