use std::sync::Arc;

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use bld_config::{BldConfig, definitions::PACKAGE_ACTION_FILE_NAME, path};
use git2::Repository;
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt};

pub struct PackageManager {
    config: Arc<BldConfig>,
}

impl PackageManager {
    pub fn new(config: Arc<BldConfig>) -> Self {
        Self { config }
    }

    pub async fn exists(&self, name: &str) -> bool {
        let encoded_name = STANDARD.encode(name.as_bytes());
        let repository_path = path![&self.config.local.packages.cache, encoded_name];
        repository_path.exists()
    }

    pub async fn get(&self, name: &str) -> Result<()> {
        let encoded_name = STANDARD.encode(name.as_bytes());
        let repository_path = path![&self.config.local.packages.cache, encoded_name];
        Repository::clone(name, &repository_path)?;
        Ok(())
    }

    pub async fn is_synced(&self, _name: &str) -> bool {
        true
    }

    pub async fn sync(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    pub async fn read(&self, name: &str) -> Result<String> {
        let encoded_name = STANDARD.encode(name.as_bytes());
        let repository_path = path![&self.config.local.packages.cache, encoded_name];
        let file_path = path![&repository_path, PACKAGE_ACTION_FILE_NAME];
        let mut handle = File::open(file_path).await?;
        let mut content = String::new();
        handle.read_to_string(&mut content).await?;
        Ok(content)
    }
}
