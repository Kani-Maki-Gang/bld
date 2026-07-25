use anyhow::{Result, anyhow, bail};
use bld_config::definitions::TOOL_DEFAULT_CONFIG;
use serde::{Serialize, de::DeserializeOwned};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use tokio::fs::{File, create_dir_all, read_to_string, remove_file};
use tokio::io::AsyncWriteExt;

pub trait IsYaml {
    fn valid_path(&self) -> bool;

    fn is_yaml(&self) -> bool;
}

impl IsYaml for Path {
    fn valid_path(&self) -> bool {
        match self.extension() {
            Some(ext) => {
                if ext != "yaml" && ext != "yml" {
                    return false;
                }
            }
            None => return false,
        }

        match self.file_name() {
            Some(name) => {
                let name = name.to_string_lossy();
                if name == format!("{TOOL_DEFAULT_CONFIG}.yaml")
                    || name == format!("{TOOL_DEFAULT_CONFIG}.yml")
                {
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
        (name.ends_with(".yaml") || name.ends_with(".yml"))
            && name != format!("{TOOL_DEFAULT_CONFIG}.yaml")
            && name != format!("{TOOL_DEFAULT_CONFIG}.yml")
    }

    fn is_yaml(&self) -> bool {
        self.file_type()
            .map(|ft| ft.is_file() && self.valid_path())
            .unwrap_or_default()
    }
}

pub async fn read_tokens<T: DeserializeOwned>(path: &Path) -> Result<T> {
    if !path.is_file() {
        bail!("file not found");
    }

    let content = read_to_string(path).await?;
    serde_json::from_str(&content).map_err(|e| anyhow!(e))
}

pub async fn write_tokens<T: Serialize>(path: &Path, tokens: T) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn valid_path_accepts_both_yaml_extensions() {
        for name in ["pipeline.yaml", "pipeline.yml", "nested/pipeline.yml"] {
            assert!(Path::new(name).valid_path(), "{name} should be valid");
        }
    }

    #[test]
    pub fn valid_path_rejects_other_extensions() {
        for name in ["pipeline.json", "pipeline.toml", "pipeline", "pipeline.ym"] {
            assert!(!Path::new(name).valid_path(), "{name} should be invalid");
        }
    }

    #[test]
    pub fn valid_path_rejects_config_files() {
        for name in ["config.yaml", "config.yml"] {
            assert!(!Path::new(name).valid_path(), "{name} should be invalid");
        }
    }

    #[test]
    pub fn valid_path_is_case_sensitive() {
        for name in ["Config.yaml", "Config.yml", "CONFIG.yml"] {
            assert!(Path::new(name).valid_path(), "{name} should be valid");
        }

        for name in ["pipeline.YAML", "pipeline.YML"] {
            assert!(!Path::new(name).valid_path(), "{name} should be invalid");
        }
    }
}
