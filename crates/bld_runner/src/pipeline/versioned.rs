use super::traits::Load;
use super::v1;
use super::v2;
use crate::validator::v1 as validator_v1;
use crate::validator::v2 as validator_v2;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use futures::Future;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Write, pin::Pin, sync::Arc};
use tracing::debug;

type DependenciesRecursiveFuture = Pin<Box<dyn Future<Output = Result<HashMap<String, String>>>>>;

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
    Version1(v1::Pipeline),
    #[serde(rename(serialize = "2", deserialize = "2"))]
    Version2(v2::Pipeline),
}

impl VersionedPipeline {
    pub fn runs_on(&self) -> &str {
        match self {
            Self::Version1(pipeline) => &pipeline.runs_on,
            Self::Version2(_) => unimplemented!(),
        }
    }

    pub async fn dependencies(
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
        name: String,
    ) -> Result<HashMap<String, String>> {
        let mut hs = Self::dependencies_recursive(config, proxy, name.clone())
            .await
            .await?;
        hs.remove(&name);
        Ok(hs)
    }

    async fn dependencies_recursive(
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
        name: String,
    ) -> DependenciesRecursiveFuture {
        Box::pin(async move {
            debug!("Parsing pipeline {name}");

            let src = proxy
                .read(&name)
                .await
                .map_err(|_| anyhow!("Pipeline {name} not found"))?;

            let pipeline = Yaml::load(&src).map_err(|e| anyhow!("{e} ({name})"))?;
            let mut set = HashMap::new();
            set.insert(name.to_string(), src);

            let local_pipelines = match pipeline {
                Self::Version1(pip) => pip.local_dependencies(config.as_ref()),
                Self::Version2(pip) => pip.local_dependencies(config.as_ref()),
            };

            for pipeline in local_pipelines.into_iter() {
                for (k, v) in Self::dependencies_recursive(config.clone(), proxy.clone(), pipeline)
                    .await
                    .await?
                {
                    set.insert(k, v);
                }
            }

            Ok(set)
        })
    }

    pub fn cron(&self) -> Option<&str> {
        if let Self::Version2(pip) = self {
            pip.cron.as_deref()
        } else {
            None
        }
    }

    pub async fn validate_with_verbose_errors(
        &self,
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
    ) -> Result<()> {
        match self {
            Self::Version1(pip) => {
                validator_v1::PipelineValidator::new(pip, config, proxy)
                    .validate()
                    .await
            }
            Self::Version2(pip) => {
                validator_v2::PipelineValidator::new(pip, config, proxy)?
                    .validate()
                    .await
            }
        }
        .map_err(|e| anyhow!("Expression errors\r\n\r\n{e}"))
    }

    pub async fn validate(
        &self,
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
    ) -> Result<()> {
        self.validate_with_verbose_errors(config, proxy)
            .await
            .map_err(|_| anyhow!("Pipeline has expression errors"))
    }
}
