use serde::{Deserialize, Serialize};

use crate::{
    pipeline::v3::Pipeline,
    traits::{IntoVariables, Variables},
};

#[cfg(feature = "all")]
use crate::traits::Dependencies;

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunnerFile {
    #[serde(rename(serialize = "pipeline", deserialize = "pipeline"))]
    PipelineFileType(Box<Pipeline>),
    #[serde(rename(serialize = "action", deserialize = "action"))]
    ActionFileType,
}

#[cfg(feature = "all")]
impl Dependencies for RunnerFile {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        match self {
            Self::PipelineFileType(pipeline) => {
                let from_steps = pipeline
                    .jobs
                    .iter()
                    .flat_map(|(_, steps)| steps)
                    .flat_map(|s| s.local_deps(config));

                let from_external = pipeline
                    .external
                    .iter()
                    .filter(|e| e.server.is_none())
                    .map(|e| e.pipeline.to_owned());

                from_steps.chain(from_external).collect()
            }

            Self::ActionFileType => vec![],
        }
    }
}

impl IntoVariables for RunnerFile {
    fn into_variables(self) -> Variables {
        match self {
            Self::PipelineFileType(p) => (p.variables, p.environment),
            Self::ActionFileType => unimplemented!(),
        }
    }
}
