use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AddJobRequest {
    pub schedule: String,
    pub pipeline: String,
    pub inputs: Option<HashMap<String, String>>,
    pub env: Option<HashMap<String, String>>,
    pub is_default: bool,
}

impl AddJobRequest {
    pub fn new(
        schedule: String,
        pipeline: String,
        inputs: Option<HashMap<String, String>>,
        env: Option<HashMap<String, String>>,
        is_default: bool,
    ) -> Self {
        Self {
            schedule,
            pipeline,
            inputs,
            env,
            is_default,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateJobRequest {
    pub id: String,
    pub schedule: String,
    pub inputs: Option<HashMap<String, String>>,
    pub env: Option<HashMap<String, String>>,
}

impl UpdateJobRequest {
    pub fn new(
        id: String,
        schedule: String,
        inputs: Option<HashMap<String, String>>,
        env: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            id,
            schedule,
            inputs,
            env,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct JobFiltersParams {
    pub id: Option<String>,
    pub pipeline: Option<String>,
    pub schedule: Option<String>,
    pub is_default: Option<bool>,
    pub limit: Option<u64>,
}

impl JobFiltersParams {
    pub fn new(
        id: Option<String>,
        pipeline: Option<String>,
        schedule: Option<String>,
        is_default: Option<bool>,
        limit: Option<u64>,
    ) -> Self {
        Self {
            id,
            pipeline,
            schedule,
            is_default,
            limit,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CronJobResponse {
    pub id: String,
    pub schedule: String,
    pub pipeline: String,
    pub inputs: Option<HashMap<String, String>>,
    pub env: Option<HashMap<String, String>>,
    pub is_default: bool,
    pub date_created: String,
    pub date_updated: Option<String>,
}
