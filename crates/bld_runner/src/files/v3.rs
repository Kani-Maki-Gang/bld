use std::collections::HashSet;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_core::fs::FileSystem;

use serde::{Deserialize, Serialize};

use crate::{
    action::v3::Action,
    pipeline::v3::Pipeline,
    traits::{IntoVariables, Variables},
};

#[cfg(feature = "all")]
use crate::traits::Dependencies;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunnerFile {
    #[serde(rename(serialize = "pipeline", deserialize = "pipeline"))]
    PipelineFileType(Box<Pipeline>),
    #[serde(rename(serialize = "action", deserialize = "action"))]
    ActionFileType(Box<Action>),
}

impl RunnerFile {
    pub fn required_inputs(&self) -> Option<HashSet<&str>> {
        match self {
            Self::PipelineFileType(pipeline) => pipeline.required_inputs(),
            Self::ActionFileType(action) => action.required_inputs(),
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
impl Dependencies for RunnerFile {
    async fn local_deps(&self, config: &BldConfig, fs: &FileSystem) -> Vec<String> {
        match self {
            Self::PipelineFileType(pipeline) => {
                let mut dependecies = vec![];
                for steps in pipeline.jobs.values() {
                    for step in steps {
                        let mut local_deps = step.local_deps(config, fs).await;
                        dependecies.append(&mut local_deps);
                    }
                }
                for external in &pipeline.external {
                    if external.server.is_none() {
                        dependecies.push(external.uses.to_owned());
                    }
                }
                dependecies
            }

            Self::ActionFileType(action) => action.local_deps(config, fs).await,
        }
    }
}
