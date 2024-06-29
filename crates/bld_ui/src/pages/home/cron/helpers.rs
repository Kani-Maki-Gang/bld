use crate::api;
use anyhow::{anyhow, Result};
use bld_models::dtos::{CronJobResponse, JobFiltersParams, PipelineInfoQueryParams};
use bld_runner::VersionedPipeline;
use leptos::*;
use std::collections::HashMap;

pub fn hash_map_rw_signals(
    pipeline_items: HashMap<String, String>,
    mut cron_items: Option<HashMap<String, String>>,
) -> HashMap<String, RwSignal<String>> {
    pipeline_items
        .into_iter()
        .map(|(k, v)| {
            let value = cron_items
                .as_mut()
                .and_then(|c| c.remove(&k))
                .unwrap_or_else(|| v);

            (k, create_rw_signal(value))
        })
        .collect()
}

pub fn hash_map_strings(items: HashMap<String, RwSignal<String>>) -> HashMap<String, String> {
    items
        .into_iter()
        .map(|(k, v)| (k, v.get_untracked()))
        .collect()
}

pub async fn get_cron(id: Option<String>) -> Result<CronJobResponse> {
    let id = id.ok_or_else(|| anyhow!("Id not provided as query parameter"))?;
    let params = JobFiltersParams {
        id: Some(id),
        ..Default::default()
    };
    api::cron(params)
        .await
        .and_then(|x| x.into_iter().next().ok_or_else(|| anyhow!("No data found")))
}

pub async fn get_pipeline(name: Option<String>) -> Result<VersionedPipeline> {
    let name = name.ok_or_else(|| anyhow!("Name not provided as query parameter"))?;
    let params = PipelineInfoQueryParams::Name { name };
    api::print(params).await
}
