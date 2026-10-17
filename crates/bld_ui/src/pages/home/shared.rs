use crate::{components::badge::RunState, format};
use bld_models::dtos::PipelineHealthKpi;

pub fn health_ticks(kpi: &PipelineHealthKpi) -> Vec<RunState> {
    kpi.runs.iter().map(|r| RunState::parse(&r.state)).collect()
}

pub fn health_state(kpi: &PipelineHealthKpi) -> RunState {
    kpi.runs
        .last()
        .map(|r| RunState::parse(&r.state))
        .unwrap_or(RunState::NoRun)
}

pub fn health_last_completed(kpi: &PipelineHealthKpi) -> RunState {
    kpi.runs
        .iter()
        .rev()
        .map(|r| RunState::parse(&r.state))
        .find(|s| s.is_done())
        .unwrap_or(RunState::NoRun)
}

pub fn health_rate(kpi: &PipelineHealthKpi) -> String {
    match kpi.success_percentage {
        Some(p) if p > 0.0 && p < 1.0 => format!("{p:.1}% success"),
        Some(p) => format!("{p:.0}% success"),
        None => "no completed runs".to_string(),
    }
}

pub fn health_last(kpi: &PipelineHealthKpi) -> String {
    kpi.last_run_date
        .as_deref()
        .map(format::ago)
        .unwrap_or_else(|| "never run".to_string())
}

pub fn pipeline_href(id: &str, name: &str) -> String {
    format!("/files/info?id={id}&name={name}")
}

pub fn plural(count: u64, one: &str, many: &str) -> String {
    if count == 1 {
        format!("{count} {one}")
    } else {
        format!("{count} {many}")
    }
}
