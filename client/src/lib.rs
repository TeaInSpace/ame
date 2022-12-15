mod grpc_client {
    tonic::include_proto!("ame.v1");
}

pub use grpc_client::*;

pub use grpc_client::{LogEntry, TaskIdentifier, TaskLogRequest};

impl From<&str> for TaskIdentifier {
    fn from(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl TaskLogRequest {
    pub fn stream_from_beginning(task_name: &str, watch: bool) -> Self {
        Self {
            taskid: Some(TaskIdentifier::from(task_name)),
            start_from: Some(1),
            watch: Some(watch),
        }
    }
}

impl CreateTaskRequest {
    pub fn new(task_name: &str, task_template: TaskTemplate) -> Self {
        Self {
            id: Some(TaskIdentifier::from(task_name)),
            template: Some(task_template),
        }
    }
}

impl TaskProjectDirectoryStructure {
    pub fn new(task_name: &str, project_id: &str, paths: Vec<String>) -> Self {
        Self {
            taskid: Some(TaskIdentifier::from(task_name)),
            projectid: project_id.to_string(),
            paths,
        }
    }
}

#[cfg(test)]
mod tests {}
