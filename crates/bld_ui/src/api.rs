use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{
    AddJobRequest, CronJobResponse, HistQueryParams, HistoryEntry, JobFiltersParams, ListResponse,
    PipelineInfoQueryParams, PipelinePathRequest, PipelineQueryParams, UpdateJobRequest,
};
use bld_runner::VersionedPipeline;
use leptos::leptos_dom::logging;
use leptos_router::{use_navigate, NavigateOptions};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
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

fn navigate_to_login() {
    let nav = use_navigate();
    nav("/login", NavigateOptions::default());
}

fn handle_error<T>(status: StatusCode, body: String) -> Result<T> {
    if status == StatusCode::UNAUTHORIZED {
        navigate_to_login();
    }
    let error = format!("Status {status} {body}");
    logging::console_error(&error);
    bail!(error)
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

fn get_authorization_header() -> Result<Option<(String, String)>> {
    let auth_available = get_auth_available()?;
    if !auth_available {
        bail!("auth not available")
    }

    let access_token = match get_access_token() {
        Ok(token) => token,
        Err(e) => {
            navigate_to_login();
            bail!(e)
        }
    };

    Ok(Some((
        "Authorization".to_owned(),
        format!("Bearer {}", access_token),
    )))
}

fn add_authorization_header(req_builder: RequestBuilder) -> Result<RequestBuilder> {
    match get_authorization_header() {
        Ok(Some((auth_header, auth_value))) => Ok(req_builder.header(auth_header, auth_value)),
        Ok(None) => Ok(req_builder),
        Err(e) => bail!(e),
    }
}

pub async fn auth_available() -> Result<Response> {
    let url = build_url("/v1/auth/available")?;
    let res = Client::builder().build()?.get(&url).send().await?;
    Ok(res)
}

pub async fn stop(id: String) -> Result<()> {
    let url = build_url("/v1/stop")?;
    let request = add_authorization_header(Client::builder().build()?.post(&url))?;
    let response = request.json(&id).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}

pub async fn cron(params: JobFiltersParams) -> Result<Vec<CronJobResponse>> {
    let url = build_url("/v1/cron")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.query(&params).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn cron_insert(data: AddJobRequest) -> Result<()> {
    let url = build_url("/v1/cron")?;
    let request = add_authorization_header(Client::builder().build()?.post(&url))?;
    let response = request.json(&data).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}

pub async fn cron_update(data: UpdateJobRequest) -> Result<()> {
    let url = build_url("/v1/cron")?;
    let request = add_authorization_header(Client::builder().build()?.patch(&url))?;
    let response = request.json(&data).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}

pub async fn cron_delete(id: String) -> Result<()> {
    let url = build_url(format!("/v1/cron/{id}"))?;
    let request = add_authorization_header(Client::builder().build()?.delete(&url))?;
    let response = request.json(&id).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}

pub async fn list() -> Result<Vec<ListResponse>> {
    let url = build_url("/v1/list")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.header("Accept", "application/json").send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn hist(params: HistQueryParams) -> Result<Vec<HistoryEntry>> {
    let url = build_url("/v1/hist")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.query(&params).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn print(params: PipelineInfoQueryParams) -> Result<VersionedPipeline> {
    let url = build_url("/v1/print")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn run(data: RunParams) -> Result<String> {
    let url = build_url("/v1/run")?;
    let request = add_authorization_header(Client::builder().build()?.post(&url))?;
    let response = request.json(&data).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn pipeline_move(params: PipelinePathRequest) -> Result<()> {
    let url = build_url("/v1/move")?;
    let request = add_authorization_header(Client::builder().build()?.patch(&url))?;
    let response = request.json(&params).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}

pub async fn remove(params: PipelineQueryParams) -> Result<()> {
    let url = build_url("/v1/remove")?;
    let request = add_authorization_header(Client::builder().build()?.delete(&url))?;
    let response = request.query(&params).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}

pub async fn copy(params: PipelinePathRequest) -> Result<()> {
    let url = build_url("/v1/copy")?;
    let request = add_authorization_header(Client::builder().build()?.post(&url))?;
    let response = request.json(&params).send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}
