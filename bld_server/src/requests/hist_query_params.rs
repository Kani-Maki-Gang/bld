use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct HistQueryParams {
    pub state: String,
    pub limit: i64,
}
