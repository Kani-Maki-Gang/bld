use serde::{Deserialize, Serialize};

#[cfg(feature = "all")]
use bld_config::BldConfig;

#[cfg(feature = "all")]
use bld_utils::fs::IsYaml;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,

    #[serde(default)]
    pub exec: Vec<BuildStepExec>,
}

impl BuildStep {
    #[cfg(feature = "all")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildStepExec {
    Shell(String),

    External {
        #[serde(rename(serialize = "ext", deserialize = "ext"))]
        value: String,
    },
}
