use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiInfo {
    pub count: u64,
    pub percentage: f64,
}

impl KpiInfo {
    pub fn new(count: u64, percentage: f64) -> Self {
        Self { count, percentage }
    }
}
