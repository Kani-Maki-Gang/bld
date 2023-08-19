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

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelinePathRequest {
    pub pipeline: String,
    pub target: String,
}

impl PipelinePathRequest {
    pub fn new(pipeline: &str, target: &str) -> Self {
        Self {
            pipeline: pipeline.to_string(),
            target: target.to_string(),
        }
    }
}
