use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ArtifactsV1 {
    pub method: String,
    pub from: String,
    pub to: String,
    pub ignore_errors: Option<bool>,
    pub after: Option<String>,
}
