use anyhow::{anyhow, bail, Result};
use bld_config::definitions::TOOL_DEFAULT_CONFIG;
use serde::{Deserialize, Serialize};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, read_to_string, remove_file, File};
use tokio::io::AsyncWriteExt;

pub trait IsYaml {
    fn valid_path(&self) -> bool;

    fn is_yaml(&self) -> bool;
}

impl IsYaml for Path {
    fn valid_path(&self) -> bool {
        match self.extension() {
            Some(ext) => {
                if ext != "yaml" {
                    return false;
                }
            }
            None => return false,
        }

        match self.file_name() {
            Some(name) => {
                if name.to_string_lossy() == format!("{TOOL_DEFAULT_CONFIG}.yaml") {
                    return false;
                }
            }
            None => return false,
        }

        true
    }

    fn is_yaml(&self) -> bool {
        self.is_file() && self.valid_path()
    }
}

impl IsYaml for PathBuf {
    fn valid_path(&self) -> bool {
        let path = self.as_path();
        path.valid_path()
    }

    fn is_yaml(&self) -> bool {
        let path = self.as_path();
        path.is_yaml()
    }
}

impl IsYaml for DirEntry {
    fn valid_path(&self) -> bool {
        let name = self.file_name();
        let name = name.to_string_lossy();
        name.ends_with(".yaml") && name != format!("{TOOL_DEFAULT_CONFIG}.yaml")
    }

    fn is_yaml(&self) -> bool {
        self.file_type()
            .map(|ft| ft.is_file() && self.valid_path())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

impl AuthTokens {
    pub fn new(access_token: String, refresh_token: Option<String>) -> Self {
        Self {
            access_token,
            refresh_token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenParams {
    pub refresh_token: String,
}

impl RefreshTokenParams {
    pub fn new(refresh_token: &str) -> Self {
        Self {
            refresh_token: refresh_token.to_owned(),
        }
    }
}

pub async fn read_tokens(path: &Path) -> Result<AuthTokens> {
    if !path.is_file() {
        bail!("file not found");
    }

    let content = read_to_string(path).await?;
    serde_json::from_str(&content).map_err(|e| anyhow!(e))
}

pub async fn write_tokens(path: &Path, tokens: AuthTokens) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).await?;
    }

    if path.is_file() {
        remove_file(path).await?;
    }

    let data = serde_json::to_vec(&tokens)?;
    File::create(path).await?.write_all(&data).await?;
    Ok(())
}
