use crate::pipeline::{v1, v2};
use crate::traits::{IntoVariables, Variables};
use serde::{Deserialize, Serialize};

use super::v3 as files_v3;

#[cfg(feature = "all")]
use std::collections::HashMap;
use std::collections::HashSet;

#[cfg(feature = "all")]
use crate::traits::{Dependencies, Load};

#[cfg(feature = "all")]
use crate::validator::v1 as validator_v1;

#[cfg(feature = "all")]
use crate::validator::v2 as validator_v2;

#[cfg(feature = "all")]
use crate::validator::v3::{self as validator_v3, ConsumeValidator};

#[cfg(feature = "all")]
use anyhow::{anyhow, Result};

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_core::fs::FileSystem;

#[cfg(feature = "all")]
use futures::Future;

#[cfg(feature = "all")]
use std::{fmt::Write, pin::Pin, sync::Arc};

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
type DependenciesRecursiveFuture = Pin<Box<dyn Future<Output = Result<HashMap<String, String>>>>>;

#[cfg(feature = "all")]
pub struct Yaml;

#[cfg(feature = "all")]
impl Load<VersionedFile> for Yaml {
    fn load(input: &str) -> Result<VersionedFile> {
        serde_yaml_ng::from_str(input).map_err(|_| anyhow!("Pipeline file has syntax errors"))
    }

    fn load_with_verbose_errors(input: &str) -> Result<VersionedFile> {
        serde_yaml_ng::from_str(input).map_err(|e| {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "version")]
pub enum VersionedFile {
    #[serde(rename(serialize = "1", deserialize = "1"))]
    Version1(v1::Pipeline),
    #[serde(rename(serialize = "2", deserialize = "2"))]
    Version2(v2::Pipeline),
    #[serde(rename(serialize = "3", deserialize = "3"))]
    Version3(files_v3::RunnerFile),
}

impl VersionedFile {
    #[cfg(feature = "all")]
    pub async fn dependencies(
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        name: String,
    ) -> Result<HashMap<String, String>> {
        let mut hs = Self::dependencies_recursive(config, fs, name.clone())
            .await
            .await?;
        hs.remove(&name);
        Ok(hs)
    }

    #[cfg(feature = "all")]
    async fn dependencies_recursive(
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        name: String,
    ) -> DependenciesRecursiveFuture {
        use crate::traits::Dependencies;

        Box::pin(async move {
            debug!("Parsing pipeline {name}");

            let src = fs
                .read(&name)
                .await
                .map_err(|_| anyhow!("Pipeline {name} not found"))?;

            let file = Yaml::load(&src).map_err(|e| anyhow!("{e} ({name})"))?;
            let mut set = HashMap::new();
            set.insert(name.to_string(), src);
            let local_deps = file.local_deps(&config);

            for pipeline in local_deps.into_iter() {
                for (k, v) in Self::dependencies_recursive(config.clone(), fs.clone(), pipeline)
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

    pub fn required_inputs(&self) -> HashSet<&str> {
        match self {
            Self::Version1(pip) => pip.required_inputs(),
            Self::Version2(pip) => pip.required_inputs(),
            Self::Version3(file) => file.required_inputs(),
        }
    }

    #[cfg(feature = "all")]
    pub async fn validate_with_verbose_errors(
        &self,
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
    ) -> Result<()> {
        use crate::traits::Validate;

        match self {
            Self::Version1(pip) => {
                validator_v1::PipelineValidator::new(pip, config, fs)
                    .validate()
                    .await
            }

            Self::Version2(pip) => {
                validator_v2::PipelineValidator::new(pip, config, fs)?
                    .validate()
                    .await
            }

            Self::Version3(file) => {
                validator_v3::RunnerFileValidator::new(file, config, fs)?
                    .validate()
                    .await
            }
        }
        .map_err(|e| anyhow!("Expression errors\r\n\r\n{e}"))
    }

    #[cfg(feature = "all")]
    pub async fn validate(&self, config: Arc<BldConfig>, fs: Arc<FileSystem>) -> Result<()> {
        self.validate_with_verbose_errors(config, fs)
            .await
            .map_err(|_| anyhow!("Pipeline has expression errors"))
    }
}

#[cfg(feature = "all")]
impl Dependencies for VersionedFile {
    fn local_deps(&self, config: &BldConfig) -> Vec<String> {
        match self {
            Self::Version1(pip) => pip.local_deps(config),
            Self::Version2(pip) => pip.local_deps(config),
            Self::Version3(file) => file.local_deps(config),
        }
    }
}

impl IntoVariables for VersionedFile {
    fn into_variables(self) -> Variables {
        match self {
            Self::Version1(pip) => pip.into_variables(),
            Self::Version2(pip) => pip.into_variables(),
            Self::Version3(file) => file.into_variables(),
        }
    }
}
