use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
    pub variables: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
    pub is_default: bool,
    pub date_created: String,
    pub date_updated: Option<String>,
}
