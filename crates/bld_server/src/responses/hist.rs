use serde::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Serialize, Deserialize, Tabled)]
pub struct HistoryEntry {
    pub name: String,
    pub id: String,
    pub user: String,
    pub state: String,
    pub start_date_time: String,
    pub end_date_time: String,
}
