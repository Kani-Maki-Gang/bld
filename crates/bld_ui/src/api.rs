use anyhow::{anyhow, bail, Result};
use bld_models::dtos::{
    AddJobRequest, AuthTokens, CompletedPipelinesKpi, CronJobResponse, HistQueryParams,
    HistoryEntry, JobFiltersParams, ListResponse, PipelineInfoQueryParams, PipelinePathRequest,
    PipelinePerCompletedStateKpi, PipelineQueryParams, PipelineRunsPerMonthKpi, QueuedPipelinesKpi,
    RunningPipelinesKpi, RunsPerUserKpi, UpdateJobRequest,
};
use bld_runner::VersionedPipeline;
use leptos::leptos_dom::logging;
use leptos_router::{use_navigate, NavigateOptions};
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::Serialize;
use std::{collections::HashMap, fmt::Display};
use web_sys::{window, Storage};

const LOCAL_STORAGE_AUTH_AVAILABLE_KEY: &str = "auth_available";
const LOCAL_STORAGE_AUTH_TOKENS_KEY: &str = "auth_tokens";

#[derive(Serialize)]
pub enum RunParams {
    EnqueueRun {
        name: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    },
}

#[cfg(debug_assertions)]
pub fn build_url<T: Into<String> + Display>(route: T) -> Result<String> {
    Ok(format!("http://localhost:6080{route}"))
}

#[cfg(not(debug_assertions))]
pub fn build_url<T: Into<String> + Display>(route: T) -> Result<String> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;
    let origin = window
        .location()
        .origin()
        .map_err(|_| anyhow!("unable to find window origin"))?;
    Ok(format!("{origin}{route}"))
}

fn get_local_storage() -> Result<Storage> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;

    window
        .local_storage()
        .map_err(|_| anyhow!("unable to find local storage"))?
        .ok_or_else(|| anyhow!("local storage not found"))
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
    let local_storage = get_local_storage()?;
    let auth_available = local_storage
        .get(LOCAL_STORAGE_AUTH_AVAILABLE_KEY)
        .map_err(|_| anyhow!("unable to get auth_available value"))?
        .ok_or_else(|| anyhow!("auth_available value not found"))?;

    Ok(serde_json::from_str::<bool>(&auth_available)?)
}

fn set_auth_tokens(info: AuthTokens) -> Result<()> {
    let local_storage = get_local_storage()?;
    local_storage
        .set_item(
            LOCAL_STORAGE_AUTH_TOKENS_KEY,
            &serde_json::to_string(&info)?,
        )
        .map_err(|_| anyhow!("unable to set auth tokens"))
}

fn get_access_token() -> Result<String> {
    let local_storage = get_local_storage()?;
    let auth = local_storage
        .get(LOCAL_STORAGE_AUTH_TOKENS_KEY)
        .map_err(|_| anyhow!("unable to get auth tokens"))?
        .ok_or_else(|| anyhow!("auth value not found"))?;

    let info = serde_json::from_str::<AuthTokens>(&auth)?;

    Ok(info.access_token)
}

pub fn remove_auth_tokens() -> Result<()> {
    let local_storage = get_local_storage()?;
    local_storage
        .remove_item(LOCAL_STORAGE_AUTH_TOKENS_KEY)
        .map_err(|_| anyhow!("unable to remove auth tokens"))
}

fn get_authorization_header() -> Result<Option<(String, String)>> {
    let auth_available = get_auth_available()?;
    if !auth_available {
        return Ok(None);
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

pub async fn check_auth_available() -> Result<()> {
    let url = build_url("/v1/auth/available")?;
    let response = Client::builder().build()?.get(&url).send().await?;
    let status = response.status();
    let local_storage = get_local_storage()?;
    local_storage
        .set_item(
            LOCAL_STORAGE_AUTH_AVAILABLE_KEY,
            &status.is_success().to_string(),
        )
        .map_err(|_| anyhow!("unable to set auth availability"))?;
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        Ok(())
    }
}

pub fn auth_start() -> Result<()> {
    let window = window().ok_or_else(|| anyhow!("window not found"))?;
    let url = build_url("/v1/auth/web-client/start")?;
    window
        .location()
        .set_href(&url)
        .map_err(|_| anyhow!("unable to set window location"))?;
    Ok(())
}

pub async fn auth_validate(query: String) -> Result<()> {
    let url = build_url(format!("/v1/auth/web-client/validate{query}"))?;
    let request = Client::builder().build()?.get(&url);
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.text().await?)
    } else {
        set_auth_tokens(response.json::<AuthTokens>().await?)?;
        Ok(())
    }
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

pub async fn queued_pipelines() -> Result<QueuedPipelinesKpi> {
    let url = build_url("/v1/ui/kpis/queued-pipelines")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.json().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn running_pipelines() -> Result<RunningPipelinesKpi> {
    let url = build_url("/v1/ui/kpis/running-pipelines")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.json().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn completed_pipelines() -> Result<CompletedPipelinesKpi> {
    let url = build_url("/v1/ui/kpis/completed-pipelines")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.json().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn most_runs_per_user() -> Result<Vec<RunsPerUserKpi>> {
    let url = build_url("/v1/ui/kpis/most-runs-per-user")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.json().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn pipelines_per_completed_state() -> Result<Vec<PipelinePerCompletedStateKpi>> {
    let url = build_url("/v1/ui/kpis/pipelines-per-completed-state")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.json().await?)
    } else {
        Ok(response.json().await?)
    }
}

pub async fn pipeline_runs_per_month() -> Result<Vec<PipelineRunsPerMonthKpi>> {
    let url = build_url("/v1/ui/kpis/pipeline-runs-per-month")?;
    let request = add_authorization_header(Client::builder().build()?.get(&url))?;
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        handle_error(status, response.json().await?)
    } else {
        Ok(response.json().await?)
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
