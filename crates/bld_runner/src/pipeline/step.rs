use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,

    #[serde(default)]
    pub external: Vec<String>,

    #[serde(default)]
    pub exec: Vec<String>,
}

impl BuildStep {
    pub fn new(
        name: Option<String>,
        working_dir: Option<String>,
        external: Vec<String>,
        exec: Vec<String>,
    ) -> Self {
        Self {
            name,
            working_dir,
            external,
            exec,
        }
    }
}
