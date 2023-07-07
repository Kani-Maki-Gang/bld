use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PrintQueryParams {
    pub pipeline: String,
}

impl PrintQueryParams {
    pub fn new(pipeline: &str) -> Self {
        PrintQueryParams {
            pipeline: pipeline.to_owned(),
        }
    }
}
