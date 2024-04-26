#[cfg(feature = "database")]
use crate::pipeline_runs::PipelineRuns;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HistQueryParams {
    pub state: Option<String>,
    pub name: Option<String>,
    pub limit: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoryEntry {
    pub name: String,
    pub id: String,
    pub user: String,
    pub state: String,
    pub start_date_time: Option<String>,
    pub end_date_time: Option<String>,
}

impl HistoryEntry {
    pub fn display_option(value: &Option<String>) -> String {
        value.as_deref().unwrap_or("").to_string()
    }
}

#[cfg(feature = "database")]
impl From<PipelineRuns> for HistoryEntry {
    fn from(value: PipelineRuns) -> Self {
        Self {
            name: value.name,
            id: value.id,
            user: value.app_user,
            state: value.state,
            start_date_time: value.start_date.map(|x| x.format("%F %X").to_string()),
            end_date_time: value.end_date.map(|x| x.format("%F %X").to_string()),
        }
    }
}
