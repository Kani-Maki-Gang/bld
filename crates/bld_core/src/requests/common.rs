use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineQueryParams {
    pub pipeline: String,
}

impl PipelineQueryParams {
    pub fn new(pipeline: &str) -> Self {
        Self {
            pipeline: pipeline.to_owned(),
        }
    }
}
