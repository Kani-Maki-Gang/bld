use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiInfo {
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedPipelinesKpiInfo {
    pub finished_count: i64,
    pub faulted_count: i64,
    pub finished_percentage: f64,
    pub faulted_percentage: f64,
}
