pub mod taskservice {
    tonic::include_proto!("taskservice");
}

use taskservice::task_service_client::TaskServiceClient;
use taskservice::{Task, TaskIdentifier};
use tonic::{Request, Response, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TaskServiceClient::connect("http://localhost:3342").await?;

    let res = client.create_task(Request::new(Task{
        command: "test".to_string(),
        projectid: "myproject".to_string(),
    })).await?;

    let res = client
        .get_task(Request::new(TaskIdentifier {
            name: res.get_ref().name.clone(),
        }))
        .await?;

    println!("res: {:?}", res);
    Ok(())
}
