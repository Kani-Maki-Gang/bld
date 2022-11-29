use super::artifacts::ArtifactsV1;
use super::external::ExternalV1;
use super::step::BuildStepV1;
use super::traits::Load;
use anyhow::{anyhow, Result};
use bld_core::proxies::PipelineFileSystemProxy;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::debug;

pub struct Json;
pub struct Yaml;

impl Load<VersionedPipeline> for Yaml {
    fn load(input: &str) -> Result<VersionedPipeline> {
        serde_yaml::from_str(input).map_err(|e| anyhow!(e))
    }
}

impl Load<VersionedPipeline> for Json {
    fn load(input: &str) -> Result<VersionedPipeline> {
        serde_json::from_str(input).map_err(|e| anyhow!(e))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
pub enum VersionedPipeline {
    #[serde(rename(serialize = "1", deserialize = "1"))]
    Version1(PipelineV1)
}

impl VersionedPipeline {
    pub fn runs_on(&self) -> &str {
        match self {
            Self::Version1(pipeline) => &pipeline.runs_on
        }
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

        let pipeline = Yaml::load(&src)?;
        let mut set = HashMap::new();
        set.insert(name.to_string(), src);

        let local_pipelines = match pipeline {
            Self::Version1(pip) => pip.local_dependencies(),
        };

        for pipeline in local_pipelines.iter() {
            for (k, v) in Self::dependencies_recursive(proxy, &pipeline)? {
                set.insert(k, v);
            }
        }

        Ok(set)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PipelineV1 {
    pub name: Option<String>,
    pub runs_on: String,

    #[serde(default)]
    pub dispose: bool,

    #[serde(default)]
    pub environment: HashMap<String, String>,

    #[serde(default)]
    pub variables: HashMap<String, String>,

    #[serde(default)]
    pub artifacts: Vec<ArtifactsV1>,

    #[serde(default)]
    pub external: Vec<ExternalV1>,

    #[serde(default)]
    pub steps: Vec<BuildStepV1>,

}

impl PipelineV1 {
    pub fn local_dependencies(&self) -> Vec<String> {
        self.external
            .iter()
            .filter(|e| e.server.is_none())
            .map(|e| e.pipeline.to_owned())
            .collect()
    }
}
