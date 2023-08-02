use anyhow::{Context, Result};
use std::fs;

use crate::grpc::{ProjectCfg, TaskCfg};

impl ProjectCfg {
    pub fn try_from_working_dir() -> Result<Self> {
        serde_yaml::from_str(
            &fs::read_to_string("ame.yaml")
                .context("Could not read ame.yaml, are you in an AME project?")?,
        )
        .context("Could not parse ame.yaml :(")
    }

    pub fn try_from_dir(dir: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(&fs::read_to_string(format!(
            "{}/ame.yaml",
            dir
        ))?)?)
    }

    pub fn task_names(&self) -> Vec<String> {
        self.tasks.iter().filter_map(|t| t.name.clone()).collect()
    }

    pub fn get_task_cfg(&self, name: &str) -> Option<TaskCfg> {
        self.tasks
            .iter()
            .find(|t| t.name.clone().map(|n| n == name).unwrap_or(false))
            .cloned()
    }
}
