use crate::{Error, Result};

use ame::{
    custom_resources::{
        new_task::{self, Task, TaskBuilder},
        project::{self, Project},
        project_source_ctrl::ProjectSrcCtrl,
        secrets::SecretCtrl,
        task_ctrl::approve_deletion,
    },
    error::AmeError,
};
use either::Either;
use futures_util::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{DeleteParams, ListParams, LogParams, PostParams},
    runtime::wait::{await_condition, conditions},
};

use kube::{Api, Client, Resource, ResourceExt};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_stream::wrappers::ReceiverStream;

use ame::grpc::{ame_service_server::AmeService, project_file_chunk::Messages};

use ame::grpc::*;

use crate::storage::{AmeFile, ObjectStorage, S3Config, S3StorageDriver};
use tracing::{debug, instrument};

use ame::ctrl::AmeKubeResourceCtrl;
use std::{collections::HashMap, sync::Arc};
use tokio::time::{sleep, Duration};
use tonic::{Code, Request, Response, Status, Streaming};

#[derive(Debug, Clone)]
pub struct AmeServiceConfig {
    pub s3config: S3Config,
    pub bucket: String,
}

#[derive(Debug, Clone)]
pub struct Service {
    tasks: Arc<Api<Task>>,
    projects: Arc<Api<project::Project>>,
    storage: Arc<ObjectStorage<S3StorageDriver>>,
    pods: Arc<Api<Pod>>,
    secret_ctrl: Arc<SecretCtrl>,
    project_src_ctrl: Arc<ProjectSrcCtrl>,
    new_tasks: Arc<Api<new_task::Task>>,
}

#[tonic::async_trait]
impl AmeService for Service {
    async fn train_model(&self, request: Request<TrainRequest>) -> Result<Response<Empty>, Status> {
        let tr = request.into_inner();

        let Some(_project) = self
            .projects
            .list(&ListParams::default())
            .await
            .unwrap()
            .into_iter()
            .find(|p| p.spec.cfg.name == tr.projectid)
        else {
            return Err(Status::from_error(Box::new(Error::MissingModel(
                tr.model_name.clone(),
            ))));
        };

        todo!();
    }

    async fn get_task(
        &self,
        request: Request<TaskIdentifier>,
    ) -> Result<Response<TaskCfg>, Status> {
        let task = self
            .tasks
            .get(&request.get_ref().name)
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(Response::new(task.spec.cfg))
    }

    async fn run_task(
        &self,
        request: Request<RunTaskRequest>,
    ) -> Result<Response<TaskIdentifier>, Status> {
        let RunTaskRequest {
            project_id: Some(ref project_id),
            task_cfg: Some(task_cfg),
        } = request.into_inner()
        else {
            todo!();
        };

        let parent_project = self.projects.get(&project_id.name).await.unwrap();
        let mut oref = parent_project.controller_owner_ref(&()).unwrap();
        oref.controller = Some(false);

        let mut task_builder = TaskBuilder::from_cfg(task_cfg.clone());
        let task = task_builder
            .set_project(project_id.name.clone())
            .add_owner_reference(oref)
            .set_name(format!(
                "{}{}local",
                project_id.name,
                task_cfg.name.unwrap()
            ))
            .clone()
            .build();

        let task_in_cluster = self
            .new_tasks
            .create(&PostParams::default(), &task)
            .await
            .map_err(|e| Status::from_error(Box::new(e)))?;

        Ok(Response::new(TaskIdentifier {
            name: task_in_cluster.name_any(),
        }))
    }

    async fn list_tasks(
        &self,
        _request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let tasks = self
            .new_tasks
            .list(&ListParams::default())
            .await
            .unwrap()
            .items;

        let mut task_statuses: HashMap<String, TaskListEntry> = HashMap::new();

        for task in tasks {
            let entry = TaskListEntry {
                status: Some(task.clone().status.unwrap_or_default()),
                time_stamp: serde_json::json!(task.creation_timestamp().unwrap()).to_string(),
            };

            task_statuses.insert(task.name_any(), entry);
        }

        Ok(Response::new(ListTasksResponse {
            tasks: task_statuses,
        }))
    }

    async fn delete_task(
        &self,
        request: Request<TaskIdentifier>,
    ) -> Result<Response<Empty>, Status> {
        self.tasks
            .delete(&request.get_ref().name, &DeleteParams::default())
            .await
            .map_or_else(
                |e| Err(Status::from_error(Box::new(e))),
                |_| Ok(Response::new(Empty {})),
            )
    }

    async fn create_task_project_directory(
        &self,
        request: Request<TaskProjectDirectoryStructure>,
    ) -> Result<Response<Empty>, Status> {
        let structure = request.into_inner();
        let TaskProjectDirectoryStructure {
            taskid: Some(task_id),
            ..
        } = structure.clone()
        else {
            return Err(Status::invalid_argument("missing Task identifier"));
        };

        self.storage
            .store_project_directory_structure(&task_id, structure)
            .await
            .map_or_else(
                |e| Err(Status::from_error(Box::new(e))),
                |_| Ok(Response::new(Empty {})),
            )
    }

    #[instrument]
    async fn upload_project_file(
        &self,
        request: Request<Streaming<ProjectFileChunk>>,
    ) -> Result<Response<Empty>, Status> {
        let mut file: AmeFile = AmeFile::default();
        let mut task_id_option: Option<TaskIdentifier> = None;

        let mut stream = request.into_inner();
        loop {
            match stream.message().await {
                Ok(Some(ProjectFileChunk {
                    messages: Some(Messages::Chunk(mut chunk)),
                })) => file.contents.append(&mut chunk.contents),
                Ok(Some(ProjectFileChunk {
                    messages: Some(Messages::Identifier(id)),
                })) => {
                    file.key = id.filepath;
                    task_id_option = Some(TaskIdentifier { name: id.taskid });
                }
                Ok(Some(ProjectFileChunk { messages: None })) => {
                    return Err(Status::invalid_argument(
                        "missing messages from ProjectFileChunk",
                    ))
                }
                Err(e) => {
                    return Err(Status::cancelled(format!(
                        "Stream was cancelled by the caller, with status: {e}"
                    )))
                }
                Ok(None) => break,
            }
        }

        let Some(task_id) = task_id_option else {
            return Err(Status::invalid_argument("missing TaskIdentifier in stream"));
        };

        if file.key.is_empty() {
            return Err(Status::invalid_argument(
                "missing ProjectFileIdentifier in stream",
            ));
        }

        //TODO: is an empty file valid?
        debug!("uploading file: {}", file.key);

        self.storage
            .write_task_project_file(&task_id, file)
            .await
            .map_or_else(
                |e| Err(Status::from_error(Box::new(e))),
                |_| Ok(Response::new(Empty {})),
            )
    }

    type StreamTaskLogsStream = ReceiverStream<Result<LogEntry, Status>>;

    async fn stream_task_logs(
        &self,
        request: Request<TaskLogRequest>,
    ) -> Result<Response<Self::StreamTaskLogsStream>, Status> {
        let (log_sender, log_receiver) = mpsc::channel(1);
        let task_name = request.get_ref().clone().taskid.unwrap().name;
        let pods = self.pods.clone();
        let tasks = self.new_tasks.clone();

        debug!("streaming logs for: {}", &task_name);
        let _handle: JoinHandle<Result<()>> = tokio::spawn(async move {
            let mut seen_pods: Vec<String> = vec![];
            loop {
                // TODO: move away from polling
                sleep(Duration::from_secs(5)).await;
                let task_pods = pods
                    .list(&ListParams::default().labels(&format!("ame-task={task_name}")))
                    .await
                    .unwrap();

                // TODO: ensure the pod is actually done before  giving up on logging.
                for pod in &task_pods.items {
                    debug!("found pod {} for task {}", &pod.name_any(), &task_name,);

                    if seen_pods.contains(&pod.name_any()) {
                        continue;
                    }
                    seen_pods.push(pod.name_any());

                    let status = pods.get_status(&pod.name_any()).await.unwrap_or_default();

                    let phase = status.status.unwrap_or_default().phase.unwrap_or_default();
                    let mut log_params = LogParams::default();
                    log_params.container = Some("main".to_string());

                    if phase == "Failed" || phase == "Succeeded" {
                        let logs = pods.logs(&pod.name_any(), &log_params).await?;
                        debug!("sending full logs");
                        let log_entry = LogEntry {
                            contents: logs.into_bytes(),
                        };
                        log_sender
                            .send(Ok(log_entry))
                            .await
                            .or(Err(Error::TokioSendError(format!(
                                "failed to send log entry: {}",
                                pod.name_any()
                            ))))?;

                        continue;
                    }

                    debug!("waiting for pod to start running {}", pod.name_any());
                    // TODO: we end up waiting here in cases where we shouldn't. After the task has already completed.
                    if let Err(_e) = tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        await_condition(
                            Api::<Pod>::clone(&pods),
                            &pod.name_any(),
                            conditions::is_pod_running(),
                        ),
                    )
                    .await
                    {
                        debug!(
                            "failed to wait for running pod within timeout {}",
                            pod.name_any()
                        )
                    };

                    let mut pod_log_stream = pods
                        .log_stream(
                            &pod.name_any(),
                            &LogParams {
                                container: Some("main".to_string()),
                                follow: true,
                                since_seconds: Some(1),

                                ..LogParams::default()
                            },
                        )
                        .await?;

                    debug!("Streaming logs for {}", pod.name_any());
                    while let Some(e) = pod_log_stream.next().await {
                        let log_entry = LogEntry {
                            contents: e.unwrap().to_vec(),
                        };
                        log_sender.send(Ok(log_entry.clone())).await.or(Err(
                            Error::TokioSendError(format!(
                                "failed to send log entry: {}",
                                String::from_utf8(log_entry.contents)?
                            )),
                        ))?
                    }

                    debug!("finished streaming logs for {}", pod.name_any());
                }

                let Ok(task) = tasks.get_status(&task_name).await else {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                };

                debug!("checking task status {} {:?}", task.name_any(), task.status);

                match task
                    .status
                    .clone()
                    .unwrap_or_default()
                    .phase
                    .unwrap_or(task_status::Phase::Pending(TaskPhasePending {}))
                {
                    task_status::Phase::Failed(TaskPhaseFailed { .. })
                    | task_status::Phase::Succeeded(TaskPhaseSucceeded { .. }) => {
                        debug!("task has finished, stopping stream");
                        return Ok(());
                    }
                    _ => (),
                }
                sleep(Duration::from_millis(50)).await;
            }
        });

        Ok(Response::new(ReceiverStream::new(log_receiver)))
    }

    #[instrument]
    async fn remove_task(
        &self,
        request: Request<RemoveTaskRequest>,
    ) -> Result<Response<Empty>, Status> {
        let RemoveTaskRequest { name, approve } = request.into_inner();

        if approve.unwrap_or(false) {
            approve_deletion(&self.tasks, &name).await?;
        }

        let res = self
            .tasks
            .delete(&name, &DeleteParams::default())
            .await
            .map_err(AmeError::KubeApi)?;

        match res {
            Either::Left(t) => {
                if t.spec.deletion_approved {
                    return Ok(Response::new(Empty {}));
                }

                return Err(Status::new(
                    Code::FailedPrecondition,
                    "Task is not approved for deletion",
                ));
            }
            Either::Right(e) => {
                if e.is_failure() {
                    return Err(Status::failed_precondition("sfsd"));
                }
            }
        };

        Ok(Response::new(Empty {}))
    }

    #[instrument]
    async fn create_project(
        &self,
        request: Request<CreateProjectRequest>,
    ) -> Result<Response<ProjectId>, Status> {
        let request = request.into_inner();

        let Some(cfg) = request.cfg else {
            return Err(AmeError::MissingProjectcfg.into());
        };

        let mut project = Project::from_cfg(cfg);

        project.spec.enable_triggers = Some(request.enable_triggers.unwrap_or(false));

        let name = self
            .projects
            .create(&PostParams::default(), &project)
            .await
            .map_err(AmeError::KubeApi)?
            .name_any();

        Ok(Response::new(ProjectId { name }))
    }

    #[instrument]
    async fn create_project_src(
        &self,
        request: Request<ProjectSourceCfg>,
    ) -> Result<Response<ProjectSourceId>, Status> {
        let id = self.project_src_ctrl.create(request.into_inner()).await?;

        Ok(Response::new(id))
    }

    async fn create_resource(
        &self,
        request: Request<ResourceCfg>,
    ) -> Result<Response<ResourceId>, Status> {
        if let ResourceCfg {
            cfg: Some(resource_cfg::Cfg::ProjectSrcCfg(_cfg)),
        } = request.into_inner()
        {};

        Ok(Response::new(ResourceId {
            id: Some(resource_id::Id::ProjectSrcId(ProjectSourceId {
                name: "test".to_string(),
            })),
        }))
    }

    async fn update_project_src(
        &self,
        request: Request<ProjectSrcPatchRequest>,
    ) -> Result<Response<Empty>, Status> {
        let ProjectSrcPatchRequest { id, cfg } = request.into_inner();

        let Some(id) = id else {
            return Err(AmeError::MissingRequestParameter("id".to_string()).into());
        };

        let Some(cfg) = cfg else {
            return Err(AmeError::MissingRequestParameter("cfg".to_string()).into());
        };

        self.project_src_ctrl.update(id, cfg).await?;

        Ok(Response::new(Empty {}))
    }

    async fn get_project_src_cfg(
        &self,
        request: Request<ProjectSourceId>,
    ) -> Result<Response<ProjectSourceCfg>, Status> {
        Ok(Response::new(
            self.project_src_ctrl
                .get(request.into_inner())
                .await?
                .spec
                .cfg,
        ))
    }

    async fn get_project_src_status(
        &self,
        request: Request<ProjectSourceId>,
    ) -> Result<Response<ProjectSourceStatus>, Status> {
        Ok(Response::new(
            self.project_src_ctrl
                .get_status(request.into_inner())
                .await?,
        ))
    }

    type WatchProjectSrcStream = ReceiverStream<Result<ProjectSourceStatus, Status>>;

    #[instrument]
    async fn watch_project_src(
        &self,
        request: Request<ProjectSourceId>,
    ) -> Result<Response<Self::WatchProjectSrcStream>, Status> {
        Ok(Response::new(
            self.project_src_ctrl.watch(request.into_inner()).await?,
        ))
    }

    #[instrument]
    async fn delete_project_src(
        &self,
        request: Request<ProjectSourceId>,
    ) -> Result<Response<Empty>, Status> {
        debug!("deleting project src");
        self.project_src_ctrl.delete(request.into_inner()).await?;

        Ok(Response::new(Empty {}))
    }

    async fn create_secret(&self, request: Request<AmeSecret>) -> Result<Response<Empty>, Status> {
        let AmeSecret { ref key, value } = request.into_inner();
        self.secret_ctrl.store_secret(key, value).await?;

        Ok(Response::new(Empty {}))
    }

    async fn delete_secret(
        &self,
        request: Request<AmeSecretId>,
    ) -> Result<Response<Empty>, Status> {
        self.secret_ctrl
            .delete_secret(&request.into_inner().key)
            .await?;

        Ok(Response::new(Empty {}))
    }

    async fn list_secrets(&self, _request: Request<Empty>) -> Result<Response<AmeSecrets>, Status> {
        Ok(Response::new(AmeSecrets {
            secrets: self.secret_ctrl.list_secrets().await?,
        }))
    }

    #[instrument]
    async fn list_resource(
        &self,
        request: Request<ResourceListParams>,
    ) -> Result<Response<ResourceIds>, Status> {
        if let ResourceListParams {
            params: Some(resource_list_params::Params::ProjectSourceListParams(_params)),
        } = request.into_inner()
        {
            debug!("listing resource: ProjectSource",);
            let res = Ok(Response::new(ResourceIds {
                ids: self
                    .project_src_ctrl
                    .list_project_src()
                    .await?
                    .into_iter()
                    .map(|psid| ResourceId {
                        id: Some(resource_id::Id::ProjectSrcId(psid)),
                    })
                    .collect(),
            }));

            debug!("sending response");

            res
        } else {
            todo!()
        }
    }

    async fn list_project_srcs(
        &self,
        _request: Request<ProjectSourceListParams>,
    ) -> Result<Response<ListProjectSrcsResponse>, Status> {
        let cfgs = self
            .project_src_ctrl
            .list(None)
            .await?
            .into_iter()
            .map(|r| r.spec.cfg)
            .collect();

        Ok(Response::new(ListProjectSrcsResponse { cfgs }))
    }

    #[instrument]
    async fn get_project_src_id(
        &self,
        request: Request<ProjectSrcIdRequest>,
    ) -> Result<Response<ProjectSourceId>, Status> {
        Ok(Response::new(
            self.project_src_ctrl
                .get_project_src_for_repo(&request.into_inner().repo)
                .await?
                .name_any()
                .into(),
        ))
    }
}

impl Service {
    /// This method initializes a Service with the required clients and configuration.
    /// It will fail if a `Kubernetes` cluster is not reachable.
    pub async fn try_init(cfg: AmeServiceConfig) -> Result<Service> {
        let client = Client::try_default().await?;
        let target_namespace = "ame-system";
        let tasks = Api::<Task>::namespaced(client.clone(), target_namespace);
        let new_tasks = Api::<new_task::Task>::namespaced(client.clone(), target_namespace);
        let pods = Api::<Pod>::namespaced(client.clone(), target_namespace);
        let projects = Api::<Project>::namespaced(client.clone(), target_namespace);

        let task_service = Service {
            tasks: Arc::new(tasks),
            pods: Arc::new(pods),
            new_tasks: Arc::new(new_tasks),
            projects: Arc::new(projects),
            storage: Arc::new(ObjectStorage::<S3StorageDriver>::new_s3_storage(
                &cfg.bucket,
                cfg.s3config,
            )?),
            secret_ctrl: Arc::new(SecretCtrl::new(client.clone(), target_namespace)),
            project_src_ctrl: Arc::new(ProjectSrcCtrl::new(client, target_namespace)),
        };

        Ok(task_service)
    }

    pub async fn prepare_environment(&self) -> Result<()> {
        match self.storage.ensure_storage_is_ready().await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("failed to prepare object storage: {}", e);
                Err(e)
            }
        }
    }
}
