use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub id: String,
    pub pipeline: String,
}
