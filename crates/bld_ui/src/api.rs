use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{
    AddJobRequest, HistQueryParams, JobFiltersParams, PipelineInfoQueryParams, PipelinePathRequest,
    PipelineQueryParams, UpdateJobRequest,
};
use reqwest::{Client, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use web_sys::window;

#[derive(Serialize, Deserialize)]
struct AuthInfo {
    access_token: String,
    refresh_token: String,
}

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

fn get_auth_available() -> Result<bool> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;

    let local_storage = window
        .local_storage()
        .map_err(|_| anyhow!("unable to find local storage"))?
        .ok_or_else(|| anyhow!("local storage not found"))?;

    let auth_available = local_storage
        .get("auth_available")
        .map_err(|_| anyhow!("unable to get auth_available value"))?
        .ok_or_else(|| anyhow!("auth_available value not found"))?;

    Ok(serde_json::from_str::<bool>(&auth_available)?)
}

fn get_access_token() -> Result<String> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;

    let local_storage = window
        .local_storage()
        .map_err(|_| anyhow!("unable to find local storage"))?
        .ok_or_else(|| anyhow!("local storage not found"))?;

    let auth = local_storage
        .get("auth")
        .map_err(|_| anyhow!("unable to get auth value"))?
        .ok_or_else(|| anyhow!("auth value not found"))?;

    let info = serde_json::from_str::<AuthInfo>(&auth)?;

    Ok(info.access_token)
}

fn get_authorization_header() -> Result<(String, String)> {
    let auth_available = get_auth_available()?;
    if !auth_available {
        bail!("auth not available")
    }
    let access_token = get_access_token()?;
    Ok((
        "Authorization".to_owned(),
        format!("Bearer {}", access_token),
    ))
}

fn add_authorization_header(req_builder: RequestBuilder) -> RequestBuilder {
    if let Ok((auth_header, auth_value)) = get_authorization_header() {
        req_builder.header(auth_header, auth_value)
    } else {
        req_builder
    }
}

pub async fn auth_available() -> Result<Response> {
    let url = build_url("/v1/auth/available")?;
    let res = Client::builder().build()?.get(&url).send().await?;
    Ok(res)
}

pub async fn stop(id: String) -> Result<Response> {
    let url = build_url("/v1/stop")?;
    let mut res = Client::builder().build()?.post(&url);
    res = add_authorization_header(res);
    Ok(res.json(&id).send().await?)
}

pub async fn cron(params: JobFiltersParams) -> Result<Response> {
    let url = build_url("/v1/cron")?;
    let mut res = Client::builder().build()?.get(&url);
    res = add_authorization_header(res);
    Ok(res.query(&params).send().await?)
}

pub async fn cron_insert(data: AddJobRequest) -> Result<Response> {
    let url = build_url("/v1/cron")?;
    let mut res = Client::builder().build()?.post(&url);
    res = add_authorization_header(res);
    Ok(res.json(&data).send().await?)
}

pub async fn cron_update(data: UpdateJobRequest) -> Result<Response> {
    let url = build_url("/v1/cron")?;
    let mut res = Client::builder().build()?.patch(&url);
    res = add_authorization_header(res);
    Ok(res.json(&data).send().await?)
}

pub async fn cron_delete(id: String) -> Result<Response> {
    let url = build_url(format!("/v1/cron/{id}"))?;
    let mut res = Client::builder().build()?.delete(&url);
    res = add_authorization_header(res);
    Ok(res.json(&id).send().await?)
}

pub async fn list() -> Result<Response> {
    let url = build_url("/v1/list")?;
    let mut res = Client::builder().build()?.get(&url);
    res = add_authorization_header(res);
    Ok(res.header("Accept", "application/json").send().await?)
}

pub async fn hist(params: HistQueryParams) -> Result<Response> {
    let url = build_url("/v1/hist")?;
    let mut res = Client::builder().build()?.get(&url);
    res = add_authorization_header(res);
    Ok(res.query(&params).send().await?)
}

pub async fn print(params: PipelineInfoQueryParams) -> Result<Response> {
    let url = build_url("/v1/print")?;
    let mut res = Client::builder().build()?.get(&url);
    res = add_authorization_header(res);
    Ok(res
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await?)
}

pub async fn run(data: RunParams) -> Result<Response> {
    let url = build_url("/v1/run")?;
    let mut res = Client::builder().build()?.post(&url);
    res = add_authorization_header(res);
    Ok(res.json(&data).send().await?)
}

pub async fn pipeline_move(params: PipelinePathRequest) -> Result<Response> {
    let url = build_url("/v1/move")?;
    let mut res = Client::builder().build()?.patch(&url);
    res = add_authorization_header(res);
    Ok(res.json(&params).send().await?)
}

pub async fn remove(params: PipelineQueryParams) -> Result<Response> {
    let url = build_url("/v1/remove")?;
    let mut res = Client::builder().build()?.delete(&url);
    res = add_authorization_header(res);
    Ok(res.query(&params).send().await?)
}

pub async fn copy(params: PipelinePathRequest) -> Result<Response> {
    let url = build_url("/v1/copy")?;
    let mut res = Client::builder().build()?.post(&url);
    res = add_authorization_header(res);
    Ok(res.json(&params).send().await?)
}
