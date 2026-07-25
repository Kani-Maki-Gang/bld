mod auth;
pub mod definitions;
mod docker;
mod local;
mod packages;
mod path;
mod server;
mod ssh;
mod supervisor;
mod tls;

pub use auth::*;
pub use docker::*;
pub use local::*;
pub use packages::*;
pub use path::*;
use serde_yaml_ng::to_string;
pub use server::*;
pub use ssh::*;
pub use supervisor::*;
pub use tls::*;

use crate::definitions::{
    LOCAL_DEFAULT_DB_DIR, LOCAL_DEFAULT_DB_NAME, LOCAL_MACHINE_TMP_DIR, LOCAL_SERVER_HOST,
    LOCAL_SERVER_PORT, REMOTE_SERVER_AUTH, TOOL_DEFAULT_CONFIG_FILE, TOOL_DEFAULT_CONFIG_FILE_YML,
    TOOL_DIR, WEB_CLIENT_DEBUG_ORIGIN,
};
use anyhow::{Error, Result, anyhow};
use openidconnect::core::CoreClient;
use serde::{Deserialize, Serialize};
use std::env::current_dir;
use std::path::{Path, PathBuf};

#[cfg(feature = "tokio")]
use tokio::fs::read_to_string;

#[cfg(feature = "tokio")]
use tracing::debug;

pub fn err_server_not_in_config() -> Error {
    anyhow!("server not found in config")
}

pub fn err_no_server_in_config() -> Error {
    anyhow!("no server found in config")
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BldConfig {
    #[serde(skip_serializing, skip_deserializing)]
    pub root_dir: String,

    #[serde(skip_serializing, skip_deserializing)]
    pub project_dir: String,

    #[serde(default)]
    pub local: BldLocalConfig,

    #[serde(default)]
    pub remote: Vec<BldRemoteServerConfig>,
}

impl BldConfig {
    fn config_file_in_dir(dir: &Path) -> Option<PathBuf> {
        let yaml = path![dir, TOOL_DEFAULT_CONFIG_FILE];
        let yml = path![dir, TOOL_DEFAULT_CONFIG_FILE_YML];

        match (yaml.is_file(), yml.is_file()) {
            (true, true) => {
                eprintln!(
                    "warning: both {TOOL_DEFAULT_CONFIG_FILE} and {TOOL_DEFAULT_CONFIG_FILE_YML} found in {}, loading {TOOL_DEFAULT_CONFIG_FILE}",
                    dir.display()
                );
                Some(yaml)
            }
            (true, false) => Some(yaml),
            (false, true) => Some(yml),
            (false, false) => None,
        }
    }

    pub fn path() -> Result<PathBuf> {
        let mut current = current_dir()?;
        loop {
            let bld_dir = path![&current, TOOL_DIR];

            if let Some(cfg_file) = Self::config_file_in_dir(&bld_dir) {
                return Ok(cfg_file);
            }

            current = current
                .parent()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| anyhow!(".bld directory not found"))?;
        }
    }

    #[cfg(feature = "tokio")]
    pub async fn load() -> Result<Self> {
        use serde_yaml_ng::from_str;

        let path = Self::path()?;

        let root_dir = path
            .parent()
            .ok_or_else(|| anyhow!("unable to resolve config path"))?;

        let project_dir = root_dir
            .parent()
            .ok_or_else(|| anyhow!("unable to resolve project path"))?;

        debug!("loading config file from: {}", &path.display());

        let content = read_to_string(&path).await?;
        let mut instance: Self = from_str(&content).map_err(|e| anyhow!(e))?;

        instance.root_dir = root_dir
            .to_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("unable to construct config path"))?;

        instance.project_dir = project_dir
            .to_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| anyhow!("unable to construct project path"))?;

        instance.local.debug_info();
        Ok(instance)
    }

    pub fn default_yaml_for_server() -> Result<String> {
        let mut instance = Self::default();
        let default_db = path![
            &current_dir()?,
            TOOL_DIR,
            LOCAL_DEFAULT_DB_DIR,
            LOCAL_DEFAULT_DB_NAME
        ]
        .display()
        .to_string();
        instance.local.server.db = Some(format!("sqlite://{default_db}"));
        let yaml = to_string(&instance)?;
        Ok(yaml)
    }

    pub fn default_yaml_for_client() -> Result<String> {
        let mut instance = Self::default();
        instance.remote.push(BldRemoteServerConfig {
            name: "local".to_string(),
            host: LOCAL_SERVER_HOST.to_string(),
            port: LOCAL_SERVER_PORT,
            tls: false,
        });
        let yaml = to_string(&instance)?;
        Ok(yaml)
    }

    pub fn server(&self, name: &str) -> Result<&BldRemoteServerConfig> {
        self.remote
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(err_server_not_in_config)
    }

    pub fn nth_server(&self, i: usize) -> Result<&BldRemoteServerConfig> {
        self.remote.get(i).ok_or_else(err_no_server_in_config)
    }

    pub fn ssh(&self, name: &str) -> Result<&SshConfig> {
        self.local
            .ssh
            .get(name)
            .ok_or_else(|| anyhow!("ssh configuration with name '{name}' wasn't found"))
    }

    pub fn registry(&self, name: &str) -> Option<&RegistryConfig> {
        self.local.registries.get(name)
    }

    pub async fn openid_core_client(&self) -> Result<Option<CoreClient>> {
        if let Some(auth) = &self.local.server.auth {
            auth.core_client(&self.local.server.base_url_http())
                .await
                .map(Some)
        } else {
            Ok(None)
        }
    }

    pub async fn openid_web_core_client(&self) -> Result<Option<CoreClient>> {
        let Some(auth) = &self.local.server.auth else {
            return Ok(None);
        };

        if cfg!(debug_assertions) {
            auth.web_core_client(WEB_CLIENT_DEBUG_ORIGIN)
                .await
                .map(Some)
        } else {
            auth.web_core_client(&self.local.server.base_url_http())
                .await
                .map(Some)
        }
    }

    pub fn server_pipelines(&self) -> PathBuf {
        path![&self.root_dir, &self.local.server.pipelines]
    }

    pub fn log_full_path(&self, id: &str) -> PathBuf {
        path![&self.root_dir, &self.local.server.logs, id]
    }

    pub fn auth_full_path(&self, server: &str) -> PathBuf {
        path![&self.root_dir, REMOTE_SERVER_AUTH, server]
    }

    pub fn server_auth_full_path(&self, server: &str) -> Result<PathBuf> {
        self.server(server).map(|s| self.auth_full_path(&s.name))
    }

    pub fn config_full_path(&self) -> PathBuf {
        Self::config_file_in_dir(Path::new(&self.root_dir))
            .unwrap_or_else(|| path![&self.root_dir, TOOL_DEFAULT_CONFIG_FILE])
    }

    pub fn full_path(&self, name: &str) -> PathBuf {
        path![&self.root_dir, name]
    }

    pub fn tmp_full_path(&self, name: &str) -> PathBuf {
        path![&self.root_dir, LOCAL_MACHINE_TMP_DIR, name]
    }

    pub fn artifacts_run_dir(&self, run_id: &str) -> PathBuf {
        path![&self.root_dir, &self.local.artifacts, run_id]
    }

    pub fn artifact_full_path(&self, run_id: &str, name: &str) -> PathBuf {
        path![self.artifacts_run_dir(run_id), format!("{name}.tar.gz")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, create_dir_all, remove_dir_all};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = path![std::env::temp_dir(), format!("bld_config_test_{name}")];
            let _ = remove_dir_all(&path);
            create_dir_all(&path).expect("unable to create temp dir");
            Self(path)
        }

        fn touch(&self, name: &str) -> PathBuf {
            let path = path![&self.0, name];
            File::create(&path).expect("unable to create file");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = remove_dir_all(&self.0);
        }
    }

    #[test]
    fn config_file_in_dir_finds_yaml() {
        let dir = TempDir::new("finds_yaml");
        let expected = dir.touch(TOOL_DEFAULT_CONFIG_FILE);
        assert_eq!(BldConfig::config_file_in_dir(&dir.0), Some(expected));
    }

    #[test]
    fn config_file_in_dir_finds_yml() {
        let dir = TempDir::new("finds_yml");
        let expected = dir.touch(TOOL_DEFAULT_CONFIG_FILE_YML);
        assert_eq!(BldConfig::config_file_in_dir(&dir.0), Some(expected));
    }

    #[test]
    fn config_file_in_dir_prefers_yaml_over_yml() {
        let dir = TempDir::new("prefers_yaml");
        let expected = dir.touch(TOOL_DEFAULT_CONFIG_FILE);
        dir.touch(TOOL_DEFAULT_CONFIG_FILE_YML);
        assert_eq!(BldConfig::config_file_in_dir(&dir.0), Some(expected));
    }

    #[test]
    fn config_file_in_dir_ignores_other_files() {
        let dir = TempDir::new("ignores_others");
        dir.touch("config.json");
        dir.touch("default.yaml");
        assert_eq!(BldConfig::config_file_in_dir(&dir.0), None);
    }
}
