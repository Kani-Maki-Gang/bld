use crate::config::definitions::TOOL_DEFAULT_CONFIG;
use std::fs::DirEntry;
use std::path::Path;

pub trait IsYaml {
    fn is_yaml(&self) -> bool;
}

impl IsYaml for Path {
    fn is_yaml(&self) -> bool {
        if self.is_file() {
            if let Some(ext) = self.extension() {
                return ext == "yaml";
            }
        }
        false
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
