use crate::config::definitions::REMOTE_SERVER_OAUTH2;
use crate::config::Auth;
use crate::helpers::term::print_error;
use crate::path;
use actix::{Arbiter, System};
use actix_http::Payload;
use actix_web::client::Client;
use actix_web::dev::Decompress;
use actix_web::error::PayloadError;
use actix_web::web::Bytes;
use awc::http::StatusCode;
use awc::ClientResponse;
use futures::Stream;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;

type ServerResponse =
    ClientResponse<Decompress<Payload<Pin<Box<dyn Stream<Item = Result<Bytes, PayloadError>>>>>>>;

fn handle_body(body: &Result<Bytes, PayloadError>) -> String {
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
    if !res.is_empty() {
        println!("{}", res);
    }
}

async fn parse_response(resp: &mut ServerResponse) -> String {
    let body = resp.body().await;
    match resp.status() {
        StatusCode::OK => handle_body(&body),
        StatusCode::BAD_REQUEST => handle_body(&body),
        StatusCode::UNAUTHORIZED => String::from("unauthorized"),
        _ => String::from("unexpected response from server"),
    }
}

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

pub fn http_post<T>(sys: String, url: String, headers: HashMap<String, String>, body: T) -> String
where
    T: 'static + Serialize,
{
    let system = System::new(&sys);
    let rc_resp = Rc::new(RefCell::new(String::new()));
    let rc_resp_clone = Rc::clone(&rc_resp);
    Arbiter::spawn(async move {
        let client = Client::default();
        let mut request = client.post(url);
        for (key, value) in headers.iter() {
            request = request.header(&key[..], &value[..]);
        }
        let mut response = request.header("User-Agent", "Bld").send_json(&body).await;
        rc_resp_clone
            .borrow_mut()
            .push_str(&match response.as_mut() {
                Ok(r) => parse_response(r).await,
                Err(e) => e.to_string(),
            });
        System::current().stop();
    });
    let _ = system.run();
    // Capture value before drop to bypass compiler error for not lived long enough value.
    let result = rc_resp.borrow().to_string();
    result
}
