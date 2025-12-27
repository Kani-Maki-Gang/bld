use std::sync::Arc;

use anyhow::{Result, bail};
use bld_config::{BldConfig, definitions::PACKAGE_ACTION_FILE_NAME, path};
use git2::Repository;
use regex::Regex;
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt};

pub struct RepositoryInfo {
    pub url: String,
    pub name: String,
    pub branch: Option<String>,
}

pub struct PackageManager {
    config: Arc<BldConfig>,
    regex: Regex,
}

impl PackageManager {
    pub fn new(config: Arc<BldConfig>) -> Self {
        // Regex to parse git URLs (HTTPS and SSH) with optional @branch/tag
        // Examples:
        //   https://github.com/user/repo.git@branch
        //   git@github.com:user/repo.git@tag
        // Captures:
        //   1: Full URL without @branch (e.g., https://github.com/user/repo.git)
        //   2: Repository name (e.g., repo)
        //   3: Branch/tag (e.g., main)
        let regex = Regex::new(
            r"^((?:https?://[^/]+/|git@[^:]+:)(?:[^/]+/)*([^@/]+?)(?:\.git)?)(?:@(.+))?$",
        )
        .expect("Invalid regex pattern");

        Self { config, regex }
    }

    fn resolve_info(&self, source: &str) -> Result<RepositoryInfo> {
        let Some(captures) = self.regex.captures(source) else {
            bail!("Failed to parse git repository URL: {}", source);
        };

        let url = captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract repository URL"))?;

        let name = captures
            .get(2)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract repository name"))?;

        let branch = captures.get(3).map(|m| m.as_str().to_string());

        Ok(RepositoryInfo { url, name, branch })
    }

    fn repository_path(&self, info: &RepositoryInfo) -> PathBuf {
        let dir = info
            .branch
            .as_ref()
            .map(|b| format!("{}@{}", &info.name, b))
            .unwrap_or_else(|| info.name.clone());
        path![&self.config.local.packages.cache, dir]
    }

    pub async fn exists(&self, source: &str) -> bool {
        let Ok(info) = self.resolve_info(source) else {
            return false;
        };
        let repository_path = self.repository_path(&info);
        repository_path.exists()
    }

    pub async fn get(&self, source: &str) -> Result<()> {
        let info = self.resolve_info(source)?;
        let repository_path = self.repository_path(&info);
        Repository::clone(&info.url, &repository_path)?;
        Ok(())
    }

    pub async fn is_synced(&self, _name: &str) -> bool {
        true
    }

    pub async fn sync(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    pub async fn read(&self, source: &str) -> Result<String> {
        let info = self.resolve_info(source)?;
        let repository_path = self.repository_path(&info);
        let file_path = path![&repository_path, PACKAGE_ACTION_FILE_NAME];
        let mut handle = File::open(file_path).await?;
        let mut content = String::new();
        handle.read_to_string(&mut content).await?;
        Ok(content)
    }
}
