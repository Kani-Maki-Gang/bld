use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AddJobRequest {
    pub schedule: String,
    pub pipeline: String,
    pub variables: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
    pub is_default: bool,
}

impl AddJobRequest {
    pub fn new(
        schedule: String,
        pipeline: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
        is_default: bool,
    ) -> Self {
        Self {
            schedule,
            pipeline,
            variables,
            environment,
            is_default,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateJobRequest {
    pub id: String,
    pub schedule: String,
    pub variables: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
}

impl UpdateJobRequest {
    pub fn new(
        id: String,
        schedule: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            id,
            schedule,
            variables,
            environment,
        }
    }
}
