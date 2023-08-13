use futures::future::join_all;
use kube::{
    api::{ListParams, Patch, PatchParams},
    Api, ResourceExt,
};
use tracing::debug;

use super::{
    common::parent_project,
    data_set::DataSet,
    new_task::{Task, TaskContext},
    project::{generate_data_set_task_name, Project},
};

use crate::{
    custom_resources::project::{local_name, project_name},
    k8s_safe_types::ImagePullPolicy,
};

use crate::Result;

pub async fn approve_deletion(tasks: &Api<Task>, name: &str) -> Result<()> {
    let patch: Task = Task::approve_deletion_patch();

    tasks
        .patch(
            name,
            &PatchParams::apply("ame-server").force(), // TODO: should we be forcing here?
            &Patch::Apply(patch),
        )
        .await?;

    Ok(())
}

pub struct TaskCtrl {
    data_sets: Api<DataSet>,
    projects: Api<Project>,
}

impl TaskCtrl {
    pub fn new(data_sets: Api<DataSet>, projects: Api<Project>) -> Self {
        Self {
            data_sets,
            projects,
        }
    }

    pub async fn gather_task_ctx(
        &self,
        task: &Task,
        executor_image: String,
        service_account: String,
        task_image_pull_policy: ImagePullPolicy,
    ) -> Result<TaskContext> {
        debug!("gathering task context");
        // NOTE: we at least have to get datasets which this task depends on.
        let parent_project = task.project()?;

        let dependent_data_sets: Result<Vec<DataSet>> =
            join_all(task.spec.cfg.data_sets.clone().iter().map(|ds| async {
                self.resolve_data_set_ref(ds.clone(), parent_project.clone())
                    .await
            }))
            .await
            .into_iter()
            .collect();

        Ok(TaskContext {
            executor_image,
            task_image_pull_policy,
            task_volume: task.name_any(),
            required_data_sets: dependent_data_sets?,
            service_account,
        })
    }

    async fn resolve_data_set_ref(&self, ds_ref: String, root_project: String) -> Result<DataSet> {
        let project_name = if let Some(project_name) = project_name(ds_ref.clone()) {
            project_name
        } else {
            self.projects.get(&root_project).await?.spec.cfg.name
        };

        let project_objs = self.projects.list(&ListParams::default()).await?;

        let local_name = local_name(ds_ref.clone());

        let potential_ds: Vec<DataSet> = self
            .data_sets
            .list(&ListParams::default())
            .await?
            .items
            .into_iter()
            .filter(|ds| ds.spec.cfg.name == local_name)
            .filter(|ds| {
                project_objs.items.clone().iter().any(|po| {
                    ds.owner_references().iter().any(|oref| {
                        oref.kind == "Project"
                            && oref.name == po.name_any()
                            && po.spec.cfg.name == project_name
                    })
                })
            })
            .collect();

        if potential_ds.len() != 1 {
            todo!("len: {} {ds_ref}", potential_ds.len())
        }

        Ok(potential_ds[0].clone())
    }
}

pub fn resolve_data_set_path(data_set: DataSet) -> Result<String> {
    let project_name = parent_project(data_set.owner_references().to_vec())?;

    Ok(format!(
        "ame/tasks/{}/artifacts/{}",
        generate_data_set_task_name(project_name, data_set.spec.cfg.name),
        data_set.spec.cfg.path
    ))
}
