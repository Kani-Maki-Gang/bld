use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct HistQueryParams {
    pub state: Option<String>,
    pub name: Option<String>,
    pub limit: u64,
}
