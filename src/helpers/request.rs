use crate::config::definitions::REMOTE_SERVER_OAUTH2;
use crate::config::Auth;
use crate::path;
use anyhow::anyhow;
use reqwest::{Client, StatusCode};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn headers(server: &str, auth: &Auth) -> anyhow::Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    if let Auth::OAuth2(_info) = auth {
        if let Ok(token) = fs::read_to_string(path![REMOTE_SERVER_OAUTH2, server]) {
            let bearer = format!("Bearer {}", token);
            headers.insert("Authorization".to_string(), bearer);
        }
    }
    Ok(headers)
}

pub async fn get(url: String, headers: HashMap<String, String>) -> anyhow::Result<String> {
    let client = Client::new();
    let mut request = client.get(url);
    for (key, value) in headers.iter() {
        request = request.header(&key[..], &value[..]);
    }
    request = request.header("User-Agent", "Bld");
    let response = request.send().await?;
    match response.status() {
        StatusCode::OK => response.text().await.map_err(|e| anyhow!(e)),
        st => Err(anyhow!(
            "http request returned failed with status code: {}",
            st.to_string()
        )),
    }
}

pub async fn post<T>(
    url: String,
    headers: HashMap<String, String>,
    body: T,
) -> anyhow::Result<String>
where
    T: 'static + Serialize,
{
    let client = reqwest::Client::new();
    let mut request = client.post(url);
    for (key, value) in headers.iter() {
        request = request.header(&key[..], &value[..]);
    }
    request = request.header("User-Agent", "Bld");
    let response = request.json(&body).send().await?;
    match response.status() {
        StatusCode::OK => response.text().await.map_err(|e| anyhow!(e)),
        st => Err(anyhow!(
            "http request returned failed with status code: {}",
            st.to_string()
        )),
    }
}
