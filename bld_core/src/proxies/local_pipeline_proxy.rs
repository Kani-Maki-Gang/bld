#![allow(dead_code)]

use crate::proxies::PipelineFileSystemProxy;
use bld_config::{definitions::TOOL_DIR, path};
use bld_utils::fs::IsYaml;
use std::fs::{create_dir_all, read_to_string, remove_file, File};
use std::io::Write;
use std::path::PathBuf;

#[derive(Default)]
pub struct LocalPipelineProxy;

impl PipelineFileSystemProxy for LocalPipelineProxy {
    fn path(&self, name: &str) -> anyhow::Result<PathBuf> {
        Ok(path![std::env::current_dir()?, TOOL_DIR, name])
    }

    fn read(&self, name: &str) -> anyhow::Result<String> {
        let path = self.path(name)?;
        Ok(read_to_string(path)?)
    }

    fn create(&self, name: &str, content: &str) -> anyhow::Result<()> {
        let path = self.path(name)?;
        if path.is_yaml() {
            remove_file(&path)?;
        } else if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let mut handle = File::create(&path)?;
        handle.write_all(content.as_bytes())?;
        Ok(())
    }

    fn remove(&self, name: &str) -> anyhow::Result<()> {
        let path = self.path(name)?;
        if path.is_yaml() {
            remove_file(&path)?;
        }
        Ok(())
    }
}
