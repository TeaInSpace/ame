mod grpc {
    #![allow(clippy::all)]

    tonic::include_proto!("ame.v1");
}

pub use grpc::*;
pub use grpc::{LogEntry, TaskIdentifier, TaskLogRequest};

impl From<&str> for TaskIdentifier {
    fn from(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl From<String> for AmeSecretId {
    fn from(key: String) -> Self {
        AmeSecretId { key }
    }
}

impl From<String> for ProjectSourceId {
    fn from(name: String) -> Self {
        Self { name }
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

impl ProjectSourceCfg {
    pub fn git_repository(&self) -> Option<&str> {
        if let Some(GitProjectSource { ref repository, .. }) = self.git {
            Some(repository)
        } else {
            None
        }
    }
}

/*

static AME_SECRET_STORE: &str = "ame";
impl From<AmeSecret> for Secret {
    fn from(ame_secret: AmeSecret) -> Self {
        let mut secret_map = BTreeMap::new();
        secret_map.insert("secret".to_string(), ame_secret.value);

        let mut labels = BTreeMap::new();
        labels.insert("SECRET_STORE".to_string(), AME_SECRET_STORE.to_string());

        let mut secret = Secret {
            metadata: ObjectMeta {
                name: Some(key.to_string()),
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            string_data: Some(secret_map),
            ..Secret::default()
        };
    }
}*/
