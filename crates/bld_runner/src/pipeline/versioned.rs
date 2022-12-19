use super::traits::Load;
use super::validator::PipelineValidatorV1;
use super::PipelineV1;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use tracing::debug;

pub struct Yaml;

impl Load<VersionedPipeline> for Yaml {
    fn load(input: &str) -> Result<VersionedPipeline> {
        serde_yaml::from_str(input).map_err(|_| anyhow!("Pipeline file has syntax errors"))
    }

    fn load_with_verbose_errors(input: &str) -> Result<VersionedPipeline> {
        serde_yaml::from_str(input).map_err(|e| {
            let mut message = "Syntax errors".to_string();

            let _ = write!(message, "\r\n\r\n");

            if let Some(location) = e.location() {
                let _ = write!(
                    message,
                    "line: {}, column: {} ",
                    location.line(),
                    location.column()
                );
            }

            let _ = write!(message, "{e}");

            anyhow!(message)
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
pub enum VersionedPipeline {
    #[serde(rename(serialize = "1", deserialize = "1"))]
    Version1(PipelineV1),
}

impl VersionedPipeline {
    pub fn runs_on(&self) -> &str {
        match self {
            Self::Version1(pipeline) => &pipeline.runs_on,
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

        let pipeline = Yaml::load(&src).map_err(|e| anyhow!("{e} ({name})"))?;
        let mut set = HashMap::new();
        set.insert(name.to_string(), src);

        let local_pipelines = match pipeline {
            Self::Version1(pip) => pip.local_dependencies(),
        };

        for pipeline in local_pipelines.iter() {
            for (k, v) in Self::dependencies_recursive(proxy, pipeline)? {
                set.insert(k, v);
            }
        }

        Ok(set)
    }

    pub fn validate_with_verbose_errors(
        &self,
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
    ) -> Result<()> {
        match self {
            Self::Version1(pip) => PipelineValidatorV1::new(pip, config, proxy)
                .validate()
                .map_err(|e| anyhow!("Expression errors\r\n\r\n{e}")),
        }
    }

    pub fn validate(
        &self,
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
    ) -> Result<()> {
        self.validate_with_verbose_errors(config, proxy)
            .map_err(|_| anyhow!("Pipeline has expression errors"))
    }
}
