use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ListResponse {
    pub id: String,
    pub pipeline: String,
}
