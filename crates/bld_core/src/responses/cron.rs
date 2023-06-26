use serde_derive::{Deserialize, Serialize};
use tabled::Tabled;

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct CronJobResponse {
    pub id: String,
    pub schedule: String,
    pub pipeline: String,
}
