use bld_config::definitions::TOOL_DIR;
use bld_config::path;
use bld_utils::fs::IsYaml;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,

    #[serde(default)]
    pub exec: Vec<BuildStepExec>,
}

impl BuildStep {
    pub fn local_dependencies(&self) -> Vec<String> {
        self.exec
            .iter()
            .flat_map(|e| match e {
                BuildStepExec::External { value } if path![TOOL_DIR, value].is_yaml() => {
                    Some(value.to_owned())
                }
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildStepExec {
    Shell(String),

    External {
        #[serde(rename(serialize = "ext", deserialize = "ext"))]
        value: String,
    },
}
