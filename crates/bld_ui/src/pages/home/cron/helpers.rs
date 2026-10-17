use crate::api;
use anyhow::{Result, anyhow};
use bld_models::dtos::{CronJobResponse, JobFiltersParams, PipelineInfoQueryParams};
use bld_runner::VersionedFile;
use serde_yaml_ng::from_str;

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

pub async fn get_pipeline(name: Option<String>) -> Result<VersionedFile> {
    let name = name.ok_or_else(|| anyhow!("Name not provided as query parameter"))?;
    let params = PipelineInfoQueryParams::Name { name };
    let response = api::print(params).await?;
    let pipeline = from_str(&response)?;
    Ok(pipeline)
}
