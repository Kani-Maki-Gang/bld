use crate::pipeline::{v1, v2};
use crate::traits::{IntoVariables, Variables};
use serde::{Deserialize, Serialize};

use super::v3 as files_v3;

#[cfg(feature = "all")]
use std::collections::HashMap;
use std::collections::HashSet;

#[cfg(feature = "all")]
use crate::{
    traits::{Dependencies, Load},
    validator::v1 as validator_v1,
    validator::v2 as validator_v2,
    validator::v3::{self as validator_v3, ConsumeValidator},
};

#[cfg(feature = "all")]
use anyhow::{Result, anyhow, bail};

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
use git2::Repository;

#[cfg(feature = "all")]
type DependenciesRecursiveFuture = Pin<Box<dyn Future<Output = Result<HashMap<String, String>>>>>;

#[cfg(feature = "all")]
pub struct YamlLoader<'a> {
    fs: &'a FileSystem,
}

#[cfg(feature = "all")]
impl<'a> YamlLoader<'a> {
    pub fn new(fs: &'a FileSystem) -> Self {
        Self { fs }
    }
}

#[cfg(feature = "all")]
impl<'a> Load<VersionedFile> for YamlLoader<'a> {
    async fn load(&self, input: &str) -> Result<VersionedFile> {
        match self.fs.read(input).await {
            Ok(content) => {
                return serde_yaml_ng::from_str(content.as_str())
                    .map_err(|_| anyhow!("File has syntax errors"));
            }
            Err(e) => {
                debug!(
                    "failed to read file {input} due to {} trying to resolve remote repository",
                    e.to_string()
                );
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        let repo = self.fs.get_tmp_dir(&id).await;
        debug!("cloning repository to {}", repo.display().to_string());
        match Repository::clone(input, &repo) {
            Ok(_) => {
                let mut file = repo.clone();
                file.push(".bld");
                file.push("bld_action.yaml");
                debug!("loaded repository now parsing bld_action.yaml");
                let content = self.fs.read(file.display().to_string().as_str()).await?;
                let load_res = serde_yaml_ng::from_str(&content)
                    .map_err(|_| anyhow!("File has syntax errors"));
                let _ = self
                    .fs
                    .remove_tmp_dir(&repo)
                    .await
                    .inspect_err(|e| debug!("unable to clean up repository due to {e}"));
                return load_res;
            }
            Err(e) => {
                debug!(
                    "failed to clone repository {input} due to {}",
                    e.to_string()
                )
            }
        }

        bail!("unable to find either local file or remote repository")
    }

    fn load_with_verbose_errors(&self, input: &str) -> Result<VersionedFile> {
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

#[cfg(feature = "all")]
pub struct Yaml<'a> {
    fs: &'a FileSystem,
}

#[cfg(feature = "all")]
impl<'a> Yaml<'a> {
    pub fn new(fs: &'a FileSystem) -> Self {
        Self { fs }
    }
}

#[cfg(feature = "all")]
impl<'a> Load<VersionedFile> for Yaml<'a> {
    async fn load(&self, input: &str) -> Result<VersionedFile> {
        match self.fs.read(input).await {
            Ok(content) => {
                return serde_yaml_ng::from_str(content.as_str())
                    .map_err(|_| anyhow!("File has syntax errors"));
            }
            Err(e) => {
                debug!(
                    "failed to read file {input} due to {} trying to resolve remote repository",
                    e.to_string()
                );
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        let repo = self.fs.get_tmp_dir(&id).await;
        debug!("cloning repository to {}", repo.display().to_string());
        match Repository::clone(input, &repo) {
            Ok(_) => {
                let mut file = repo.clone();
                file.push(".bld");
                file.push("bld_action.yaml");
                debug!("loaded repository now parsing bld_action.yaml");
                let content = self.fs.read(file.display().to_string().as_str()).await?;
                let load_res = serde_yaml_ng::from_str(&content)
                    .map_err(|_| anyhow!("File has syntax errors"));
                let _ = self
                    .fs
                    .remove_tmp_dir(&repo)
                    .await
                    .inspect_err(|e| debug!("unable to clean up repository due to {e}"));
                return load_res;
            }
            Err(e) => {
                debug!(
                    "failed to clone repository {input} due to {}",
                    e.to_string()
                )
            }
        }

        bail!("unable to find either local file or remote repository")
    }

    fn load_with_verbose_errors(&self, input: &str) -> Result<VersionedFile> {
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

            let yaml = Yaml::new(fs.as_ref());
            let file = yaml.load(&src).await.map_err(|e| anyhow!("{e} ({name})"))?;
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

    pub fn required_inputs(&self) -> Option<HashSet<&str>> {
        match self {
            Self::Version1(_) | Self::Version2(_) => None,
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
