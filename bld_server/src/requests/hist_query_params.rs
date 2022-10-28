use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct HistQueryParams {
    pub state: String,
}
