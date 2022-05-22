use crate::config::definitions::TOOL_DEFAULT_CONFIG;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

pub trait IsYaml {
    fn is_yaml(&self) -> bool;
}

impl IsYaml for Path {
    fn is_yaml(&self) -> bool {
        if !self.is_file() {
            return false;
        }
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
}

impl IsYaml for PathBuf {
    fn is_yaml(&self) -> bool {
        let path = self.as_path();
        path.is_yaml()
    }
}

impl IsYaml for DirEntry {
    fn is_yaml(&self) -> bool {
        match self.file_type() {
            Ok(file_type) => {
                let name = self.file_name();
                let name = name.to_string_lossy();
                file_type.is_file()
                    && name.ends_with(".yaml")
                    && name != format!("{TOOL_DEFAULT_CONFIG}.yaml")
            }
            Err(_) => false,
        }
    }
}
