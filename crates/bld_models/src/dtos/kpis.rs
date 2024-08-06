use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedPipelinesKpi {
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningPipelinesKpi {
    pub count: i64,
    pub available_workers: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedPipelinesKpi {
    pub finished_count: i64,
    pub faulted_count: i64,
    pub finished_percentage: f64,
    pub faulted_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunsPerUserKpi {
    pub count: i64,
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePerCompletedStateKpi {
    pub pipeline: String,
    pub finished_percentage: f64,
    pub faulted_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRunsPerMonthKpi {
    pub month: f64,
    pub count: f64,
}
