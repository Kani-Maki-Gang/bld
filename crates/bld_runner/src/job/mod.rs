use crate::{runs_on::v3::RunsOn, step::v3::Step};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod v3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JobNeeds {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    #[serde(default = "Job::default_id")]
    pub id: String,
    pub name: Option<String>,
    pub runs_on: RunsOn,
    #[serde(rename = "if")]
    pub condition: Option<String>,
    pub needs: Option<JobNeeds>,
    pub steps: Vec<Step>,
}

impl Job {
    fn default_id() -> String {
        Uuid::new_v4().to_string()
    }
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: Self::default_id(),
            name: None,
            runs_on: RunsOn::default(),
            condition: None,
            needs: None,
            steps: vec![],
        }
    }
}
