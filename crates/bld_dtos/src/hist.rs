use bld_entities::pipeline_runs::PipelineRuns;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Serialize, Deserialize, Debug)]
pub struct HistQueryParams {
    pub state: Option<String>,
    pub name: Option<String>,
    pub limit: u64,
}

#[derive(Serialize, Deserialize, Tabled)]
pub struct HistoryEntry {
    pub name: String,
    pub id: String,
    pub user: String,
    pub state: String,
    #[tabled(display_with = "HistoryEntry::display_option")]
    pub start_date_time: Option<String>,
    #[tabled(display_with = "HistoryEntry::display_option")]
    pub end_date_time: Option<String>,
}

impl HistoryEntry {
    pub fn display_option(value: &Option<String>) -> String {
        value.as_deref().unwrap_or("").to_string()
    }
}

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
