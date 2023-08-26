use bld_config::BldConfig;
use bld_utils::fs::IsYaml;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,

    #[serde(default)]
    pub exec: Vec<BuildStepExec>,
}

impl BuildStep {
    pub fn local_dependencies(&self, config: &BldConfig) -> Vec<String> {
        self.exec
            .iter()
            .flat_map(|e| match e {
                BuildStepExec::External { value } if config.full_path(value).is_yaml() => {
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
