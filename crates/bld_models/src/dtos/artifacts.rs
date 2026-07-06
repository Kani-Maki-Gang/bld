use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtifactsQueryParams {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactResponse {
    pub id: i32,
    pub run_id: String,
    pub name: String,
    pub date_created: String,
    pub date_updated: Option<String>,
    pub date_expires: String,
}

#[cfg(feature = "database")]
impl From<crate::artifacts::Artifacts> for ArtifactResponse {
    fn from(value: crate::artifacts::Artifacts) -> Self {
        Self {
            id: value.id,
            run_id: value.run_id,
            name: value.name,
            date_created: value.date_created.format("%F %X").to_string(),
            date_updated: value.date_updated.map(|x| x.format("%F %X").to_string()),
            date_expires: value.date_expires.format("%F %X").to_string(),
        }
    }
}
