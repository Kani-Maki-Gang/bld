use bld_config::definitions::TOOL_DEFAULT_CONFIG;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

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
