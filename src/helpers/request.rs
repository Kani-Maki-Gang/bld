use crate::path;
use crate::types::Result;
use crate::config::{Auth, definitions::REMOTE_SERVER_OAUTH2};
use crate::helpers::term::print_error;
use actix::{Arbiter, System};
use actix_http::Payload;
use actix_web::{client::Client, dev::Decompress, error::PayloadError};
use awc::{http::StatusCode, ClientResponse};
use bytes::Bytes;
use futures::Stream;
use serde::Serialize;
use std::{collections::HashMap, fs, path::PathBuf, pin::Pin};

type StdResult<T, V> = std::result::Result<T, V>;
type ServerResponse = ClientResponse<
    Decompress<Payload<Pin<Box<dyn Stream<Item = StdResult<Bytes, PayloadError>>>>>>,
>;

fn handle_body(body: &StdResult<Bytes, PayloadError>) -> String {
    match body {
        Ok(b) => String::from_utf8_lossy(&b).to_string(),
        Err(e) => e.to_string(),
    }
}

async fn handle_response(resp: &mut ServerResponse) {
    let body = resp.body().await;
    let res = match resp.status() {
        StatusCode::OK => handle_body(&body),
        StatusCode::BAD_REQUEST => handle_body(&body),
        StatusCode::UNAUTHORIZED => String::from("unauthorized"),
        _ => String::from("unexpected response from server"),
    };
    if res.len() > 0 {
        println!("{}", res);
    }
}

pub fn headers(server: &str, auth: &Auth) -> Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    if let Auth::OAuth2(_info) = auth {
        let token = fs::read_to_string(path![REMOTE_SERVER_OAUTH2, server])?;
        let bearer = format!("Bearer {}", token); 
        headers.insert("Authorization".to_string(), bearer);
    }
    Ok(headers)
}

pub fn exec_get(sys: String, url: String, headers: HashMap<String, String>) {
    let system = System::new(sys);
    Arbiter::spawn(async move {
        let client = Client::default();
        let mut request = client.get(url);
        for (key, value) in headers.iter() {
            request = request.header(&key[..], &value[..]);
        }
        let mut response = request.header("User-Agent", "Bld").send().await;
        match response.as_mut() {
            Ok(resp) => handle_response(resp).await,
            Err(e) => {
                let _ = print_error(&e.to_string());
            }
        }
        System::current().stop();
    });
    let _ = system.run();
}

pub fn exec_post<T>(sys: String, url: String, headers: HashMap<String, String>, body: T)
where
    T: 'static + Serialize,
{
    let system = System::new(&sys);
    Arbiter::spawn(async move {
        let client = Client::default();
        let mut request = client.post(url);
        for (key, value) in headers.iter() {
            request = request.header(&key[..], &value[..]);
        }
        let mut response = request.header("User-Agent", "Bld").send_json(&body).await;
        match response.as_mut() {
            Ok(r) => handle_response(r).await,
            Err(e) => {
                let _ = print_error(&e.to_string());
            }
        }
        System::current().stop();
    });
    let _ = system.run();
}
