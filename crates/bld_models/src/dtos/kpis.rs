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
    pub month: i64,
    pub count: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineHealthRun {
    pub id: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineHealthKpi {
    pub id: String,
    pub pipeline: String,
    pub runs: Vec<PipelineHealthRun>,
    pub success_percentage: Option<f64>,
    pub last_run_id: Option<String>,
    pub last_run_date: Option<String>,
    pub faulted_streak: u64,
    pub faulted_streak_start: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputKpi {
    pub total: u64,
    pub cron: u64,
    pub manual: u64,
}
