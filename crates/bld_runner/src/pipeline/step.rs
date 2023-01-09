use bld_config::definitions::TOOL_DIR;
use bld_config::path;
use bld_utils::fs::IsYaml;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildStepV1 {
    pub name: Option<String>,
    pub working_dir: Option<String>,

    #[serde(default)]
    pub exec: Vec<BuildStepExecV1>,
}

impl BuildStepV1 {
    pub fn local_dependencies(&self) -> Vec<String> {
        self.exec
            .iter()
            .flat_map(|e| match e {
                BuildStepExecV1::External { value } if path![TOOL_DIR, value].is_yaml() => {
                    Some(value.to_owned())
                }
                _ => None,
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildStepExecV1 {
    Shell(String),

    External {
        #[serde(rename(serialize = "ext", deserialize = "ext"))]
        value: String,
    },
}
