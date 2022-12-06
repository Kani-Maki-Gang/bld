use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildStepV1 {
    pub name: Option<String>,
    pub working_dir: Option<String>,

    #[serde(default)]
    pub exec: Vec<BuildStepExecV1>,
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
