use crate::sync::IntoArc;
use crate::tls::load_root_certificates;
use anyhow::{anyhow, Result};
use awc::http::StatusCode;
use awc::ws::WebsocketsRequest;
use awc::{Client, ClientRequest, Connector, SendClientRequest};
use bld_config::BldRemoteServerConfig;
use bld_config::{definitions::REMOTE_SERVER_OAUTH2, path, Auth};
use rustls::ClientConfig;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tracing::debug;

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

    pub fn query<T: Serialize>(mut self, value: &T) -> Result<Self> {
        self.request = self.request.query(&value)?;
        Ok(self)
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.request = self.request.insert_header((key, value));
        self
    }

    pub fn auth(mut self, server: &BldRemoteServerConfig) -> Self {
        if let Some(Auth::OAuth2(_info)) = &server.auth {
            let path = path![REMOTE_SERVER_OAUTH2, &server.name];
            if let Ok(token) = fs::read_to_string(path) {
                let bearer = format!("Bearer {}", token.trim());
                self.request = self.request.insert_header(("Authorization", bearer));
            }
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
        let body = response.body().await.map_err(|e| anyhow!(e))?;
        let text = format!("{}", String::from_utf8_lossy(&body));

        match status {
            StatusCode::OK => {
                debug!("response from server status: {status}");
                serde_json::from_str::<T>(&text).map_err(|e| anyhow!(e))
            }
            StatusCode::BAD_REQUEST => {
                debug!("response from server status: {status}");
                Err(anyhow!(text))
            }
            st => {
                debug!("response from server status: {status}");
                Err(anyhow!(
                    "request failed with status code: {}",
                    st.to_string()
                ))
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
        if let Some(Auth::OAuth2(_)) = &server.auth {
            if let Ok(token) = server.bearer() {
                self.request = self
                    .request
                    .header("Authorization", format!("Bearer {token}"));
            }
        }
        self
    }

    pub fn request(self) -> WebsocketsRequest {
        self.request
    }
}
