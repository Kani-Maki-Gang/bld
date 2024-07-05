use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use awc::http::StatusCode;
use awc::ws::WebsocketsRequest;
use awc::{Client, ClientRequest, Connector, SendClientRequest};
use bld_config::BldConfig;
use bld_models::dtos::{
    AddJobRequest, AuthTokens, CronJobResponse, ExecClientMessage, HistQueryParams, HistoryEntry,
    JobFiltersParams, PipelineInfoQueryParams, PipelinePathRequest, PipelineQueryParams,
    PullResponse, PushInfo, RefreshTokenParams, UpdateJobRequest,
};
use bld_utils::fs::{read_tokens, write_tokens};
use bld_utils::sync::IntoArc;
use bld_utils::tls::load_root_certificates;
use rustls::ClientConfig;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::{debug, error};

#[derive(Debug)]
struct RequestError {
    text: String,
    status: StatusCode,
}

impl RequestError {
    pub fn new(text: &str, status: StatusCode) -> Self {
        Self {
            text: text.to_owned(),
            status,
        }
    }
}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "response {}: {}", self.status, self.text)
    }
}

impl Error for RequestError {}

pub struct Request {
    request: ClientRequest,
}

impl Request {
    pub fn get(url: &str) -> Self {
        Self {
            request: Client::new().get(url).insert_header(("User-Agent", "bld")),
        }
    }

    pub fn post(url: &str) -> Self {
        Self {
            request: Client::new().post(url).insert_header(("User-Agent", "bld")),
        }
    }

    pub fn patch(url: &str) -> Self {
        Self {
            request: Client::new()
                .patch(url)
                .insert_header(("User-Agent", "bld")),
        }
    }

    pub fn delete(url: &str) -> Self {
        Self {
            request: Client::new()
                .delete(url)
                .insert_header(("User-Agent", "bld")),
        }
    }

    pub fn query<T: Serialize>(mut self, value: &T) -> Result<Self> {
        self.request = self.request.query(&value)?;
        Ok(self)
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.request = self.request.insert_header((key, value));
        self
    }

    pub async fn auth(mut self, path: &Path) -> Self {
        if let Ok(tokens) = read_tokens::<AuthTokens>(path).await {
            self.request = self
                .request
                .insert_header(("Authorization", format!("Bearer {}", tokens.access_token)));
        }
        self
    }

    pub async fn text(self) -> Result<String> {
        let send_request = self.request.send();
        Self::request_with_text(send_request).await
    }

    pub async fn text_with_data<T: Serialize>(self, data: &T) -> Result<String> {
        let send_request = self.request.send_json(data);
        Self::request_with_text(send_request).await
    }

    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        let send_request = self.request.send();
        Self::request_with_json::<T>(send_request).await
    }

    pub async fn json_with_data<T, V>(self, data: &T) -> Result<V>
    where
        T: 'static + Serialize,
        V: DeserializeOwned,
    {
        let send_request = self.request.send_json(&data);
        Self::request_with_json::<V>(send_request).await
    }

    async fn request_with_text(send_request: SendClientRequest) -> Result<String> {
        let mut response = send_request.await.map_err(|e| anyhow!(e.to_string()))?;
        let status = response.status();

        match status {
            StatusCode::OK => {
                debug!("response from server status: {status}");
                response
                    .body()
                    .await
                    .map_err(|e| anyhow!(e))
                    .map(|body| String::from_utf8_lossy(&body).to_string())
            }
            StatusCode::BAD_REQUEST => {
                let body = response.body().await.map_err(|e| anyhow!(e))?;
                let text = format!("{}", String::from_utf8_lossy(&body));
                debug!("response from server status: {status}");
                Err(RequestError::new(&text, StatusCode::BAD_REQUEST).into())
            }
            StatusCode::UNAUTHORIZED => {
                debug!("response from server status: {status}");
                let message = format!("request failed with status code: {status}");
                Err(RequestError::new(&message, StatusCode::UNAUTHORIZED).into())
            }
            st => {
                debug!("response from server status: {status}");
                let message = format!("request failed with status code: {st}");
                Err(RequestError::new(&message, st).into())
            }
        }
    }

    async fn request_with_json<T: DeserializeOwned>(send_request: SendClientRequest) -> Result<T> {
        let mut response = send_request.await.map_err(|e| anyhow!(e.to_string()))?;
        let status = response.status();

        match status {
            StatusCode::OK => {
                debug!("response from server status: {status}");
                response.json::<T>().await.map_err(|e| anyhow!(e))
            }
            StatusCode::BAD_REQUEST => {
                let body = response.body().await.map_err(|e| anyhow!(e))?;
                let text = format!("{}", String::from_utf8_lossy(&body));
                debug!("response from server status: {status}");
                Err(RequestError::new(&text, StatusCode::BAD_REQUEST).into())
            }
            StatusCode::UNAUTHORIZED => {
                debug!("response from server status: {status}");
                let message = format!("request failed with status code: {status}");
                Err(RequestError::new(&message, StatusCode::UNAUTHORIZED).into())
            }
            st => {
                debug!("response from server status: {status}");
                let message = format!("request failed with status code: {st}");
                Err(RequestError::new(&message, st).into())
            }
        }
    }
}

pub struct WebSocket {
    request: WebsocketsRequest,
}

impl WebSocket {
    pub fn new(url: &str) -> Result<Self> {
        let root_certificates = load_root_certificates()?;

        let rustls_config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_certificates)
            .with_no_client_auth();

        let connector = Connector::new().rustls(rustls_config.into_arc());

        Ok(Self {
            request: Client::builder().connector(connector).finish().ws(url),
        })
    }

    pub async fn auth(mut self, path: &Path) -> Self {
        if let Ok(tokens) = read_tokens::<AuthTokens>(path).await {
            self.request = self
                .request
                .header("Authorization", format!("Bearer {}", tokens.access_token));
        }
        self
    }

    pub fn request(self) -> WebsocketsRequest {
        self.request
    }
}

pub struct HttpClient {
    base_url: String,
    auth_path: PathBuf,
}

impl HttpClient {
    pub fn new(config: Arc<BldConfig>, server: &str) -> Result<Self> {
        let server = config.server(server)?;
        let base_url = server.base_url_http();
        let auth_path = config.auth_full_path(&server.name);
        Ok(Self {
            base_url,
            auth_path,
        })
    }

    async fn refresh(&self) -> Result<()> {
        let url = format!("{}/v1/refresh", self.base_url);
        let tokens: AuthTokens = read_tokens(&self.auth_path).await?;
        let Some(refresh_token) = tokens.refresh_token else {
            error!("no refresh token found");
            bail!("request failed with status code: 401 Unauthorized");
        };
        let params = RefreshTokenParams::new(&refresh_token);
        let tokens: AuthTokens = Request::get(&url).query(&params)?.json().await?;
        write_tokens(&self.auth_path, tokens).await
    }

    fn unauthorized<T>(response: &Result<T>) -> bool {
        matches!(
            response.as_ref().map_err(|e| e.downcast_ref()),
            Err(Some(RequestError {
                status: StatusCode::UNAUTHORIZED,
                ..
            }))
        )
    }

    async fn check_inner(&self, pipeline: &str) -> Result<()> {
        let url = format!("{}/v1/check", self.base_url);
        let params = PipelineQueryParams::new(pipeline);
        Request::get(&url)
            .query(&params)?
            .auth(&self.auth_path)
            .await
            .json()
            .await
            .map(|_: String| ())
    }

    pub async fn check(&self, pipeline: &str) -> Result<()> {
        let response = self.check_inner(pipeline).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.check_inner(pipeline).await
        } else {
            response
        }
    }

    async fn deps_inner(&self, params: &PipelineQueryParams) -> Result<Vec<String>> {
        let url = format!("{}/v1/deps", self.base_url);
        Request::get(&url)
            .auth(&self.auth_path)
            .await
            .query(params)?
            .json()
            .await
    }

    pub async fn deps(&self, pipeline: &str) -> Result<Vec<String>> {
        let params = PipelineQueryParams::new(pipeline);
        let response = self.deps_inner(&params).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.deps_inner(&params).await
        } else {
            response
        }
    }

    async fn hist_inner(&self, params: &HistQueryParams) -> Result<Vec<HistoryEntry>> {
        let url = format!("{}/v1/hist", self.base_url);
        Request::get(&url)
            .query(params)?
            .auth(&self.auth_path)
            .await
            .json()
            .await
    }

    pub async fn hist(
        &self,
        state: Option<String>,
        name: Option<String>,
        limit: u64,
    ) -> Result<Vec<HistoryEntry>> {
        let params = HistQueryParams { state, name, limit };
        let response = self.hist_inner(&params).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.hist_inner(&params).await
        } else {
            response
        }
    }

    async fn print_inner(&self, params: &PipelineInfoQueryParams) -> Result<String> {
        let url = format!("{}/v1/print", self.base_url);
        Request::get(&url)
            .auth(&self.auth_path)
            .await
            .query(params)?
            .header("Accept", "text/plain")
            .text()
            .await
    }

    pub async fn print(&self, pipeline: &str) -> Result<String> {
        let params = PipelineInfoQueryParams::Name {
            name: pipeline.to_string(),
        };
        let response = self.print_inner(&params).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.print_inner(&params).await
        } else {
            response
        }
    }

    async fn list_inner(&self) -> Result<String> {
        let url = format!("{}/v1/list", self.base_url);
        Request::get(&url).auth(&self.auth_path).await.text().await
    }

    pub async fn list(&self) -> Result<String> {
        let response = self.list_inner().await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.list_inner().await
        } else {
            response
        }
    }

    async fn pull_inner(&self, params: &PipelineQueryParams) -> Result<PullResponse> {
        let url = format!("{}/v1/pull", self.base_url);
        Request::get(&url)
            .auth(&self.auth_path)
            .await
            .query(params)?
            .json()
            .await
    }

    pub async fn pull(&self, pipeline: &str) -> Result<PullResponse> {
        let params = PipelineQueryParams::new(pipeline);
        let response = self.pull_inner(&params).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.pull_inner(&params).await
        } else {
            response
        }
    }

    async fn push_inner(&self, json: &PushInfo) -> Result<()> {
        let url = format!("{}/v1/push", self.base_url);
        Request::post(&url)
            .auth(&self.auth_path)
            .await
            .json_with_data(json)
            .await
            .map(|_: String| ())
    }

    pub async fn push(&self, name: &str, content: &str) -> Result<()> {
        let json = PushInfo {
            name: name.to_owned(),
            content: content.to_owned(),
        };
        let response = self.push_inner(&json).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.push_inner(&json).await
        } else {
            response
        }
    }

    async fn remove_inner(&self, params: &PipelineQueryParams) -> Result<()> {
        let url = format!("{}/v1/remove", self.base_url);
        Request::delete(&url)
            .auth(&self.auth_path)
            .await
            .query(params)?
            .json()
            .await
            .map(|_: String| ())
    }

    pub async fn remove(&self, pipeline: &str) -> Result<()> {
        let params = PipelineQueryParams::new(pipeline);
        let response = self.remove_inner(&params).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.remove_inner(&params).await
        } else {
            response
        }
    }

    async fn run_inner(&self, json: &ExecClientMessage) -> Result<()> {
        let url = format!("{}/v1/run", self.base_url);
        Request::post(&url)
            .auth(&self.auth_path)
            .await
            .json_with_data(json)
            .await
            .map(|_: String| ())
    }

    pub async fn run(
        &self,
        pipeline: &str,
        env: Option<HashMap<String, String>>,
        vars: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let json = ExecClientMessage::EnqueueRun {
            name: pipeline.to_owned(),
            environment: env,
            variables: vars,
        };
        let response = self.run_inner(&json).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.run_inner(&json).await
        } else {
            response
        }
    }

    async fn stop_inner(&self, json: &String) -> Result<()> {
        let url = format!("{}/v1/stop", self.base_url);
        Request::post(&url)
            .auth(&self.auth_path)
            .await
            .json_with_data(json)
            .await
            .map(|_: String| ())
    }

    pub async fn stop(&self, pipeline_id: &str) -> Result<()> {
        let id = pipeline_id.to_owned();
        let response = self.stop_inner(&id).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.stop_inner(&id).await
        } else {
            response
        }
    }

    async fn cron_list_inner(&self, filters: &JobFiltersParams) -> Result<Vec<CronJobResponse>> {
        let url = format!("{}/v1/cron", self.base_url);
        Request::get(&url)
            .auth(&self.auth_path)
            .await
            .query(filters)?
            .json()
            .await
    }

    pub async fn cron_list(&self, filters: &JobFiltersParams) -> Result<Vec<CronJobResponse>> {
        let response = self.cron_list_inner(filters).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.cron_list_inner(filters).await
        } else {
            response
        }
    }

    async fn cron_add_inner(&self, body: &AddJobRequest) -> Result<()> {
        let url = format!("{}/v1/cron", self.base_url);
        Request::post(&url)
            .auth(&self.auth_path)
            .await
            .json_with_data(body)
            .await
            .map(|_: String| ())
    }

    pub async fn cron_add(&self, body: &AddJobRequest) -> Result<()> {
        let response = self.cron_add_inner(body).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.cron_add_inner(body).await
        } else {
            response
        }
    }

    async fn cron_update_inner(&self, body: &UpdateJobRequest) -> Result<()> {
        let url = format!("{}/v1/cron", self.base_url);
        Request::patch(&url)
            .auth(&self.auth_path)
            .await
            .json_with_data(body)
            .await
            .map(|_: String| ())
    }

    pub async fn cron_update(&self, body: &UpdateJobRequest) -> Result<()> {
        let response = self.cron_update_inner(body).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.cron_update_inner(body).await
        } else {
            response
        }
    }

    async fn cron_remove_inner(&self, id: &str) -> Result<()> {
        let url = format!("{}/v1/cron/{id}", self.base_url);
        Request::delete(&url)
            .auth(&self.auth_path)
            .await
            .json()
            .await
            .map(|_: String| ())
    }

    pub async fn cron_remove(&self, id: &str) -> Result<()> {
        let response = self.cron_remove_inner(id).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.cron_remove_inner(id).await
        } else {
            response
        }
    }

    async fn copy_inner(&self, data: &PipelinePathRequest) -> Result<()> {
        let url = format!("{}/v1/copy", self.base_url);
        Request::post(&url)
            .auth(&self.auth_path)
            .await
            .json_with_data(data)
            .await
            .map(|_: String| ())
    }

    pub async fn copy(&self, pipeline: &str, target: &str) -> Result<()> {
        let data = PipelinePathRequest::new(pipeline, target);
        let response = self.copy_inner(&data).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.copy_inner(&data).await
        } else {
            response
        }
    }

    async fn mv_inner(&self, data: &PipelinePathRequest) -> Result<()> {
        let url = format!("{}/v1/move", self.base_url);
        Request::patch(&url)
            .auth(&self.auth_path)
            .await
            .json_with_data(data)
            .await
            .map(|_: String| ())
    }

    pub async fn mv(&self, pipeline: &str, target: &str) -> Result<()> {
        let data = PipelinePathRequest::new(pipeline, target);
        let response = self.mv_inner(&data).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.mv_inner(&data).await
        } else {
            response
        }
    }
}
