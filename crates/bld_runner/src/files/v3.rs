use std::collections::{HashMap, HashSet};

use crate::{
    action::v3::Action,
    pipeline::v3::Pipeline,
    traits::{IntoVariables, Variables},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use {
    crate::deps::v3::{Dependencies, Dependency},
    bld_core::fs::FileSystem,
    bld_pkg::PackageManager,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunnerFile {
    #[serde(rename(serialize = "pipeline", deserialize = "pipeline"))]
    PipelineFileType(Box<Pipeline>),
    #[serde(rename(serialize = "action", deserialize = "action"))]
    ActionFileType(Box<Action>),
}

impl RunnerFile {
    pub fn env_map(&self) -> HashMap<String, String> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.env.clone(),
            Self::ActionFileType(_) => HashMap::new(),
        }
    }

    pub fn inputs_map(&self) -> HashMap<String, String> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.inputs_map(),
            Self::ActionFileType(action) => action.inputs_map(),
        }
    }

    pub fn required_inputs(&self) -> Option<HashSet<&str>> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.required_inputs(),
            Self::ActionFileType(action) => action.required_inputs(),
        }
    }

    pub fn cron(&self) -> Option<&str> {
        match self {
            Self::PipelineFileType(pip) => pip.cron.as_deref(),
            Self::ActionFileType(_) => None,
        }
    }
}

impl IntoVariables for RunnerFile {
    fn into_variables(self) -> Variables {
        match self {
            Self::PipelineFileType(p) => p.into_variables(),
            Self::ActionFileType(a) => a.into_variables(),
        }
    }
}

#[cfg(feature = "all")]
impl<'a> Dependencies<'a> for RunnerFile {
    async fn local_deps(&'a self, fs: &FileSystem) -> Vec<Dependency<'a>> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.local_deps(fs).await,
            Self::ActionFileType(action) => action.local_deps(fs).await,
        }
    }

    async fn remote_deps(&'a self, manager: &PackageManager) -> Vec<Dependency<'a>> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.remote_deps(manager).await,
            Self::ActionFileType(action) => action.remote_deps(manager).await,
        }
    }

    async fn jobs(&'a self) -> Vec<Dependency<'a>> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.jobs().await,
            Self::ActionFileType(action) => action.jobs().await,
        }
    }

    async fn all(&'a self, manager: &PackageManager, fs: &FileSystem) -> Vec<Dependency<'a>> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.all(manager, fs).await,
            Self::ActionFileType(action) => action.all(manager, fs).await,
        }
    }
}
