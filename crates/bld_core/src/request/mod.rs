use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use crate::auth::{read_tokens, write_tokens, AuthTokens, RefreshTokenParams};
use crate::messages::ExecClientMessage;
use crate::requests::{
    AddJobRequest, CheckQueryParams, HistQueryParams, JobFiltersParams, PushInfo, UpdateJobRequest,
};
use crate::responses::{CronJobResponse, HistoryEntry, PullResponse};
use anyhow::{anyhow, bail, Result};
use awc::http::StatusCode;
use awc::ws::WebsocketsRequest;
use awc::{Client, ClientRequest, Connector, SendClientRequest};
use bld_config::{BldConfig, BldRemoteServerConfig};
use bld_utils::sync::IntoArc;
use bld_utils::tls::load_root_certificates;
use rustls::ClientConfig;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tracing::debug;

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

    pub fn auth(mut self, server: &BldRemoteServerConfig) -> Self {
        if let Ok(tokens) = read_tokens(&server.name) {
            self.request = self
                .request
                .insert_header(("Authorization", format!("Bearer {}", tokens.access_token)));
        }
        self
    }

    pub async fn send<T: DeserializeOwned>(self) -> Result<T> {
        let send_request = self.request.send();
        Self::do_send::<T>(send_request).await
    }

    pub async fn send_json<T, V>(self, json: &T) -> Result<V>
    where
        T: 'static + Serialize,
        V: DeserializeOwned,
    {
        let send_request = self.request.send_json(&json);
        Self::do_send::<V>(send_request).await
    }

    async fn do_send<T: DeserializeOwned>(send_request: SendClientRequest) -> Result<T> {
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
            st => {
                debug!("response from server status: {status}");
                let message = format!("request failed with status code: {st}");
                Err(RequestError::new(&message, StatusCode::UNAUTHORIZED).into())
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

    pub fn auth(mut self, server: &BldRemoteServerConfig) -> Self {
        if let Ok(tokens) = read_tokens(&server.name) {
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
    config: Arc<BldConfig>,
    server: String,
}

impl HttpClient {
    pub fn new(config: Arc<BldConfig>, server: &str) -> Self {
        Self {
            config,
            server: server.to_owned(),
        }
    }

    async fn refresh(&self) -> Result<()> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/refresh", server.base_url_http());
        let tokens = read_tokens(&self.server)?;
        let Some(refresh_token) = tokens.refresh_token else {
            bail!("no refresh token found");
        };
        let params = RefreshTokenParams::new(&refresh_token);
        let tokens: AuthTokens = Request::get(&url).query(&params)?.send().await?;
        write_tokens(&self.server, tokens)
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
        let server = self.config.server(&self.server)?;
        let url = format!("{}/check", server.base_url_http());
        let params = CheckQueryParams {
            pipeline: pipeline.to_owned(),
        };
        Request::get(&url)
            .query(&params)?
            .auth(server)
            .send()
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

    async fn deps_inner(&self, pipeline: &String) -> Result<Vec<String>> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/deps", server.base_url_http());
        Request::post(&url).auth(server).send_json(pipeline).await
    }

    pub async fn deps(&self, pipeline: &str) -> Result<Vec<String>> {
        let pipeline = pipeline.to_owned();
        let response = self.deps_inner(&pipeline).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.deps_inner(&pipeline).await
        } else {
            response
        }
    }

    async fn hist_inner(&self, params: &HistQueryParams) -> Result<Vec<HistoryEntry>> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/hist", server.base_url_http());
        Request::get(&url).query(params)?.auth(server).send().await
    }

    pub async fn hist(
        &self,
        state: Option<String>,
        name: Option<String>,
        limit: i64,
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

    async fn inspect_inner(&self, pipeline: &String) -> Result<String> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/inspect", server.base_url_http());
        Request::post(&url).auth(server).send_json(pipeline).await
    }

    pub async fn inspect(&self, pipeline: &str) -> Result<String> {
        let pipeline = pipeline.to_owned();
        let response = self.inspect_inner(&pipeline).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.inspect_inner(&pipeline).await
        } else {
            response
        }
    }

    async fn list_inner(&self) -> Result<String> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/list", server.base_url_http());
        Request::get(&url).auth(server).send().await
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

    async fn pull_inner(&self, pipeline: &String) -> Result<PullResponse> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/pull", server.base_url_http());
        Request::post(&url).auth(server).send_json(pipeline).await
    }

    pub async fn pull(&self, pipeline: &str) -> Result<PullResponse> {
        let pipeline = pipeline.to_owned();
        let response = self.pull_inner(&pipeline).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.pull_inner(&pipeline).await
        } else {
            response
        }
    }

    async fn push_inner(&self, json: &PushInfo) -> Result<()> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/push", server.base_url_http());
        Request::post(&url)
            .auth(server)
            .send_json(json)
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

    async fn remove_inner(&self, json: &String) -> Result<()> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/remove", server.base_url_http());
        Request::delete(&url)
            .auth(server)
            .send_json(json)
            .await
            .map(|_: String| ())
    }

    pub async fn remove(&self, pipeline: &str) -> Result<()> {
        let pipeline = pipeline.to_owned();
        let response = self.remove_inner(&pipeline).await;

        if Self::unauthorized(&response) {
            self.refresh().await?;
            self.remove_inner(&pipeline).await
        } else {
            response
        }
    }

    async fn run_inner(&self, json: &ExecClientMessage) -> Result<()> {
        let server = self.config.server(&self.server)?;
        let url = format!("{}/run", server.base_url_http());
        Request::post(&url)
            .auth(server)
            .send_json(json)
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
        let server = self.config.server(&self.server)?;
        let url = format!("{}/stop", server.base_url_http());
        Request::post(&url)
            .auth(server)
            .send_json(json)
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
        let server = self.config.server(&self.server)?;
        let url = format!("{}/cron", server.base_url_http());
        Request::get(&url).auth(server).query(filters)?.send().await
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
        let server = self.config.server(&self.server)?;
        let url = format!("{}/cron", server.base_url_http());
        Request::post(&url)
            .auth(server)
            .send_json(body)
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
        let server = self.config.server(&self.server)?;
        let url = format!("{}/cron", server.base_url_http());
        Request::patch(&url)
            .auth(server)
            .send_json(body)
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
        let server = self.config.server(&self.server)?;
        let url = format!("{}/cron/{id}", server.base_url_http());
        Request::delete(&url)
            .auth(server)
            .send()
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
}
