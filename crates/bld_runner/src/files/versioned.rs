use crate::pipeline::{v1, v2};
use crate::traits::{IntoVariables, Variables};
use serde::{Deserialize, Serialize};

use super::v3 as files_v3;

#[cfg(feature = "all")]
use std::collections::HashMap;
use std::collections::HashSet;

#[cfg(feature = "all")]
use crate::{
    traits::Dependencies,
    validator::v1 as validator_v1,
    validator::v2 as validator_v2,
    validator::v3::{self as validator_v3, ConsumeValidator},
};

#[cfg(feature = "all")]
use anyhow::{Result, anyhow};

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_core::fs::FileSystem;

#[cfg(feature = "all")]
use bld_pkg::PackageManager;

#[cfg(feature = "all")]
use futures::Future;

#[cfg(feature = "all")]
use std::{fmt::Write, pin::Pin, sync::Arc};

#[cfg(feature = "all")]
use tracing::debug;

#[cfg(feature = "all")]
type DependenciesRecursiveFuture = Pin<Box<dyn Future<Output = Result<HashMap<String, String>>>>>;

#[cfg(feature = "all")]
pub enum VersionedFileSource {
    LocalFile,
    Package,
}

#[cfg(feature = "all")]
pub struct VersionedFileLoader<'a> {
    package_manager: &'a PackageManager,
    fs: &'a FileSystem,
    verbose: bool,
}

#[cfg(feature = "all")]
impl<'a> VersionedFileLoader<'a> {
    pub fn new(package_manager: &'a PackageManager, fs: &'a FileSystem, verbose: bool) -> Self {
        Self {
            package_manager,
            fs,
            verbose,
        }
    }

    fn parse_content(&self, content: &str) -> Result<VersionedFile> {
        serde_yaml_ng::from_str(content).map_err(|e| {
            if self.verbose {
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
            } else {
                anyhow!("File has syntax errors")
            }
        })
    }

    async fn load_local(&self, name: &str) -> Result<String> {
        self.fs.read(name).await.map_err(|e| anyhow!(e))
    }

    async fn load_package(&self, name: &str) -> Result<String> {
        if self.package_manager.exists(name) {
            self.package_manager.try_sync(name).await?;
        } else {
            self.package_manager.get(name).await?;
        }
        self.package_manager
            .read(name)
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn get_source(&self, name: &str) -> Option<VersionedFileSource> {
        use bld_utils::fs::IsYaml;

        if self
            .fs
            .path(name)
            .await
            .map(|x| x.is_yaml())
            .unwrap_or_default()
        {
            return Some(VersionedFileSource::LocalFile);
        }

        if self.package_manager.is_package(name) {
            return Some(VersionedFileSource::Package);
        }

        None
    }

    pub async fn load(&self, name: &str) -> Result<VersionedFileMetadata> {
        let source = self
            .get_source(name)
            .await
            .ok_or_else(|| anyhow!("Unable to deduce file source (local file or package)"))?;

        let raw = match source {
            VersionedFileSource::LocalFile => self.load_local(name).await,
            VersionedFileSource::Package => self.load_package(name).await,
        }?;

        let file = self.parse_content(&raw)?;

        Ok(VersionedFileMetadata::new(raw, file))
    }
}

pub struct VersionedFileMetadata {
    pub raw: String,
    pub file: VersionedFile,
}

impl VersionedFileMetadata {
    pub fn new(raw: String, file: VersionedFile) -> Self {
        Self { raw, file }
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
        package_manager: Arc<PackageManager>,
        name: String,
    ) -> Result<HashMap<String, String>> {
        let mut hs = Self::dependencies_recursive(config, fs, package_manager, name.clone())
            .await
            .await?;
        hs.remove(&name);
        Ok(hs)
    }

    #[cfg(feature = "all")]
    async fn dependencies_recursive(
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
        name: String,
    ) -> DependenciesRecursiveFuture {
        use crate::traits::Dependencies;

        Box::pin(async move {
            debug!("Parsing pipeline {name}");
            let loader = VersionedFileLoader::new(&package_manager, &fs, false);
            let metadata = loader
                .load(&name)
                .await
                .map_err(|e| anyhow!("{e} ({name})"))?;
            let mut set = HashMap::new();
            set.insert(name.to_string(), metadata.raw.clone());
            let local_deps = metadata.file.local_deps(&config, &fs).await;
            for pipeline in local_deps.into_iter() {
                for (k, v) in Self::dependencies_recursive(
                    config.clone(),
                    fs.clone(),
                    package_manager.clone(),
                    pipeline,
                )
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
        match self {
            Self::Version1(_) => None,
            Self::Version2(pip) => pip.cron.as_deref(),
            Self::Version3(file) => file.cron(),
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
        package_manager: Arc<PackageManager>,
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
                validator_v3::RunnerFileValidator::new(file, config, fs, package_manager)?
                    .validate()
                    .await
            }
        }
        .map_err(|e| anyhow!("Expression errors\r\n\r\n{e}"))
    }

    #[cfg(feature = "all")]
    pub async fn validate(
        &self,
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
    ) -> Result<()> {
        self.validate_with_verbose_errors(config, fs, package_manager)
            .await
            .map_err(|_| anyhow!("Pipeline has expression errors"))
    }
}

#[cfg(feature = "all")]
impl Dependencies for VersionedFile {
    async fn local_deps(&self, config: &BldConfig, fs: &FileSystem) -> Vec<String> {
        match self {
            Self::Version1(pip) => pip.local_deps(config, fs).await,
            Self::Version2(pip) => pip.local_deps(config, fs).await,
            Self::Version3(file) => file.local_deps(config, fs).await,
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
