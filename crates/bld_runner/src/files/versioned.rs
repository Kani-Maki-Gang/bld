use crate::pipeline::{v1, v2};
use crate::traits::{IntoVariables, Variables};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::v3 as files_v3;

#[cfg(feature = "all")]
use {
    crate::{
        deps::v3::Dependencies as Dependencies_v3,
        traits::Dependencies,
        validator::v1 as validator_v1,
        validator::v2 as validator_v2,
        validator::v3::{self as validator_v3, ConsumeValidator},
    },
    anyhow::{Result, anyhow, bail},
    bld_config::BldConfig,
    bld_core::fs::FileSystem,
    bld_models::dtos::FileType,
    bld_pkg::PackageManager,
    std::collections::HashMap,
    std::{fmt::Write, sync::Arc},
    tracing::debug,
};

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
        let raw = match self.load_local(name).await {
            Ok(raw) => raw,
            Err(local_error) => self.load_package(name).await.map_err(|package_error| {
                debug!("unable to load {name} as a local file due to {local_error}");
                debug!("unable to load {name} as a package due to {package_error}");
                anyhow!("Unable to identify value as local action or package")
            })?,
        };

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
        let loader = VersionedFileLoader::new(&package_manager, &fs, false);
        let mut files = HashMap::new();
        let mut pending = vec![name.clone()];

        while let Some(current) = pending.pop() {
            if files.contains_key(&current) {
                continue;
            }
            debug!("Parsing pipeline {current}");
            let metadata = loader
                .load(&current)
                .await
                .map_err(|e| anyhow!("{e} ({current})"))?;
            pending.extend(metadata.file.local_deps(&config, &fs).await);
            files.insert(current, metadata.raw);
        }

        files.remove(&name);
        Ok(files)
    }

    #[cfg(feature = "all")]
    pub async fn check_dependency_cycles(
        config: &BldConfig,
        fs: &FileSystem,
        package_manager: &PackageManager,
        name: &str,
    ) -> Result<()> {
        let loader = VersionedFileLoader::new(package_manager, fs, false);
        let local_deps = async |name: &str| match loader.load(name).await {
            Ok(metadata) => metadata.file.local_deps(config, fs).await.into_iter(),
            Err(_) => Vec::new().into_iter(),
        };

        let mut visited = HashSet::from([name.to_owned()]);
        let mut path = vec![(name.to_owned(), local_deps(name).await)];

        while let Some((_, deps)) = path.last_mut() {
            let Some(dep) = deps.next() else {
                path.pop();
                continue;
            };

            if let Some(start) = path.iter().position(|(file, _)| *file == dep) {
                let cycle: Vec<&str> = path[start..]
                    .iter()
                    .map(|(file, _)| file.as_str())
                    .chain([dep.as_str()])
                    .collect();
                bail!("Cyclic dependency detected: {}", cycle.join(" -> "));
            }

            if visited.insert(dep.clone()) {
                let deps = local_deps(&dep).await;
                path.push((dep, deps));
            }
        }

        Ok(())
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
                validator_v3::RunnerFileValidator::new(file, config, fs, package_manager)
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

    #[cfg(feature = "all")]
    async fn local_deps(&self, config: &BldConfig, fs: &FileSystem) -> Vec<String> {
        match self {
            Self::Version1(pip) => pip.local_deps(config, fs).await,
            Self::Version2(pip) => pip.local_deps(config, fs).await,
            Self::Version3(file) => file
                .local_deps(fs)
                .await
                .into_iter()
                .flat_map(|x| x.get_local())
                .map(|x| x.to_owned())
                .collect(),
        }
    }

    #[cfg(feature = "all")]
    pub fn file_type(&self) -> FileType {
        match self {
            VersionedFile::Version3(files_v3::RunnerFile::ActionFileType(_)) => FileType::Action,
            _ => FileType::Pipeline,
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

#[cfg(all(test, feature = "all"))]
mod tests {
    use super::{VersionedFile, VersionedFileLoader};
    use bld_config::{BldConfig, BldPackages};
    use bld_core::fs::FileSystem;
    use bld_pkg::PackageManager;
    use std::{path::PathBuf, sync::Arc};

    struct TestEnv {
        dir: PathBuf,
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
        package_manager: Arc<PackageManager>,
    }

    impl TestEnv {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("bld_runner_versioned_{name}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let mut config = BldConfig {
                root_dir: dir.to_string_lossy().into_owned(),
                ..Default::default()
            };
            config.local.packages = BldPackages {
                cache: dir.join("packages").to_string_lossy().into_owned(),
                strict_sync: false,
            };
            let config = Arc::new(config);
            Self {
                dir,
                fs: Arc::new(FileSystem::local(config.clone())),
                package_manager: Arc::new(PackageManager::new(config.clone())),
                config,
            }
        }

        async fn load_error(&self, name: &str) -> String {
            VersionedFileLoader::new(&self.package_manager, &self.fs, false)
                .load(name)
                .await
                .err()
                .unwrap()
                .to_string()
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[tokio::test]
    async fn load_reports_a_missing_local_file_with_a_directory() {
        let env = TestEnv::new("missing_nested");
        assert_eq!(
            env.load_error("actions/missing.yaml").await,
            "Unable to identify value as local action or package"
        );
    }

    #[tokio::test]
    async fn load_reports_a_missing_local_file() {
        let env = TestEnv::new("missing");
        assert_eq!(
            env.load_error("missing.yaml").await,
            "Unable to identify value as local action or package"
        );
    }

    #[tokio::test]
    async fn load_reads_an_existing_local_file() {
        let env = TestEnv::new("existing");
        let raw =
            "version: 3\ntype: action\nname: probe\nsteps:\n  - id: noop\n    run: echo noop\n";
        std::fs::write(env.dir.join("existing.yaml"), raw).unwrap();
        let metadata = VersionedFileLoader::new(&env.package_manager, &env.fs, false)
            .load("existing.yaml")
            .await
            .unwrap();
        assert_eq!(metadata.raw, raw);
    }

    fn write_uses(env: &TestEnv, name: &str, uses: &[&str]) {
        let mut raw = format!("version: 3\ntype: action\nname: {name}\nsteps:\n");
        for (i, file) in uses.iter().enumerate() {
            raw.push_str(&format!("  - id: s{i}\n    uses: {file}\n"));
        }
        if uses.is_empty() {
            raw.push_str("  - id: noop\n    run: echo noop\n");
        }
        std::fs::write(env.dir.join(name), raw).unwrap();
    }

    async fn dependencies(env: &TestEnv, name: &str) -> Vec<String> {
        let mut deps: Vec<String> = VersionedFile::dependencies(
            env.config.clone(),
            env.fs.clone(),
            env.package_manager.clone(),
            name.to_owned(),
        )
        .await
        .unwrap()
        .into_keys()
        .collect();
        deps.sort();
        deps
    }

    async fn cycle_error(env: &TestEnv, name: &str) -> Option<String> {
        VersionedFile::check_dependency_cycles(&env.config, &env.fs, &env.package_manager, name)
            .await
            .err()
            .map(|e| e.to_string())
    }

    #[tokio::test]
    async fn dependencies_of_a_self_referencing_file_are_empty() {
        let env = TestEnv::new("deps_self");
        write_uses(&env, "a.yaml", &["a.yaml"]);
        assert!(dependencies(&env, "a.yaml").await.is_empty());
    }

    #[tokio::test]
    async fn dependencies_of_a_cycle_contain_each_file_once() {
        let env = TestEnv::new("deps_cycle");
        write_uses(&env, "a.yaml", &["b.yaml"]);
        write_uses(&env, "b.yaml", &["a.yaml", "c.yaml"]);
        write_uses(&env, "c.yaml", &[]);
        assert_eq!(dependencies(&env, "a.yaml").await, vec!["b.yaml", "c.yaml"]);
    }

    #[tokio::test]
    async fn check_dependency_cycles_reports_a_self_reference() {
        let env = TestEnv::new("cycle_self");
        write_uses(&env, "a.yaml", &["a.yaml"]);
        assert_eq!(
            cycle_error(&env, "a.yaml").await.unwrap(),
            "Cyclic dependency detected: a.yaml -> a.yaml"
        );
    }

    #[tokio::test]
    async fn check_dependency_cycles_reports_an_indirect_cycle() {
        let env = TestEnv::new("cycle_indirect");
        write_uses(&env, "a.yaml", &["b.yaml"]);
        write_uses(&env, "b.yaml", &["c.yaml"]);
        write_uses(&env, "c.yaml", &["b.yaml"]);
        assert_eq!(
            cycle_error(&env, "a.yaml").await.unwrap(),
            "Cyclic dependency detected: b.yaml -> c.yaml -> b.yaml"
        );
    }

    #[tokio::test]
    async fn check_dependency_cycles_accepts_a_shared_dependency() {
        let env = TestEnv::new("cycle_diamond");
        write_uses(&env, "a.yaml", &["b.yaml", "c.yaml"]);
        write_uses(&env, "b.yaml", &["d.yaml"]);
        write_uses(&env, "c.yaml", &["d.yaml"]);
        write_uses(&env, "d.yaml", &[]);
        assert!(cycle_error(&env, "a.yaml").await.is_none());
    }
}
