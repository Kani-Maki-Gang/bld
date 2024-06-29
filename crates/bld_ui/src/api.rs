use anyhow::{anyhow, Result};
use bld_models::dtos::{
    AddJobRequest, HistQueryParams, JobFiltersParams, PipelineInfoQueryParams, PipelinePathRequest,
    PipelineQueryParams, UpdateJobRequest,
};
use reqwest::{Client, Response};
use serde::Serialize;
use std::{collections::HashMap, fmt::Display};
use web_sys::window;

#[derive(Serialize)]
pub enum RunParams {
    EnqueueRun {
        name: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    },
}

pub fn build_url<T: Into<String> + Display>(route: T) -> Result<String> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;
    let origin = window
        .location()
        .origin()
        .map_err(|_| anyhow!("unable to find window origin"))?;
    Ok(format!("{origin}{route}"))
}

#[allow(dead_code)]
fn get_bearer_token() -> Result<String> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;

    let local_storage = window
        .local_storage()
        .map_err(|_| anyhow!("unable to find local storage"))?
        .ok_or_else(|| anyhow!("local storage not found"))?;

    let _auth = local_storage
        .get("auth")
        .map_err(|_| anyhow!("unable to get auth value"))?;

    Ok(String::new())
}

pub async fn auth_available() -> Result<Response> {
    let url = build_url("/v1/auth/available")?;
    let res = Client::builder().build()?.get(&url).send().await?;
    Ok(res)
}

pub async fn stop(id: String) -> Result<Response> {
    let url = build_url("/v1/stop")?;
    let res = Client::builder()
        .build()?
        .post(&url)
        .json(&id)
        .send()
        .await?;
    Ok(res)
}

pub async fn cron(params: JobFiltersParams) -> Result<Response> {
    let url = build_url("/v1/cron")?;
    let res = Client::builder()
        .build()?
        .get(&url)
        .query(&params)
        .send()
        .await?;
    Ok(res)
}

pub async fn cron_insert(data: AddJobRequest) -> Result<Response> {
    let url = build_url("/v1/cron")?;
    let res = Client::builder()
        .build()?
        .post(&url)
        .json(&data)
        .send()
        .await?;
    Ok(res)
}

pub async fn cron_update(data: UpdateJobRequest) -> Result<Response> {
    let url = build_url("/v1/cron")?;
    let res = Client::builder()
        .build()?
        .patch(&url)
        .json(&data)
        .send()
        .await?;
    Ok(res)
}

pub async fn cron_delete(id: String) -> Result<Response> {
    let url = build_url(format!("/v1/cron/{id}"))?;
    let res = Client::builder()
        .build()?
        .delete(&url)
        .json(&id)
        .send()
        .await?;
    Ok(res)
}

pub async fn list() -> Result<Response> {
    let url = build_url("/v1/list")?;
    let res = Client::builder()
        .build()?
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await?;
    Ok(res)
}

pub async fn hist(params: HistQueryParams) -> Result<Response> {
    let url = build_url("/v1/hist")?;
    let res = Client::builder()
        .build()?
        .get(&url)
        .query(&params)
        .send()
        .await?;
    Ok(res)
}

pub async fn print(params: PipelineInfoQueryParams) -> Result<Response> {
    let url = build_url("/v1/print")?;
    let res = Client::builder()
        .build()?
        .get(&url)
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await?;
    Ok(res)
}

pub async fn run(data: RunParams) -> Result<Response> {
    let url = build_url("/v1/run")?;
    let res = Client::builder()
        .build()?
        .post(&url)
        .json(&data)
        .send()
        .await?;
    Ok(res)
}

pub async fn pipeline_move(params: PipelinePathRequest) -> Result<Response> {
    let url = build_url("/v1/move")?;
    let res = Client::builder()
        .build()?
        .patch(&url)
        .json(&params)
        .send()
        .await?;
    Ok(res)
}

pub async fn remove(params: PipelineQueryParams) -> Result<Response> {
    let url = build_url("/v1/remove")?;
    let res = Client::builder()
        .build()?
        .delete(&url)
        .query(&params)
        .send()
        .await?;
    Ok(res)
}

pub async fn copy(params: PipelinePathRequest) -> Result<Response> {
    let url = build_url("/v1/copy")?;
    let res = Client::builder()
        .build()?
        .post(&url)
        .json(&params)
        .send()
        .await?;
    Ok(res)
}
