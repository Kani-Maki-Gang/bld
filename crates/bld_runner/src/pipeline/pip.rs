use crate::pipeline::artifacts::Artifacts;
use crate::pipeline::external::External;
use crate::pipeline::step::BuildStep;
use anyhow::{anyhow, Result};
use bld_core::proxies::PipelineFileSystemProxy;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: String,

    #[serde(default)]
    pub dispose: bool,

    #[serde(default)]
    pub environment: HashMap<String, String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub artifacts: Vec<Artifacts>,

    #[serde(default)]
    pub external: Vec<External>,

    #[serde(default)]
    pub steps: Vec<BuildStep>,
}

impl Pipeline {
    pub fn parse(src: &str) -> Result<Pipeline> {
        serde_yaml::from_str(src).map_err(|e| anyhow!(e))
    }

    pub fn dependencies(
        proxy: &PipelineFileSystemProxy,
        name: &str,
    ) -> Result<HashMap<String, String>> {
        Self::dependencies_recursive(proxy, name).map(|mut hs| {
            hs.remove(name);
            hs.into_iter().collect()
        })
    }

    fn dependencies_recursive(
        proxy: &PipelineFileSystemProxy,
        name: &str,
    ) -> Result<HashMap<String, String>> {
        debug!("Parsing pipeline {name}");

        let src = proxy
            .read(name)
            .map_err(|_| anyhow!("Pipeline {name} not found"))?;

        let pipeline = Pipeline::parse(&src)?;
        let mut set = HashMap::new();
        set.insert(name.to_string(), src);

        for external in pipeline.external.iter() {
            if external.server.is_none() {
                let subset = Self::dependencies_recursive(proxy, &external.pipeline)?;
                for (k, v) in subset {
                    set.insert(k, v);
                }
            }
        }

        Ok(set)
    }
}
