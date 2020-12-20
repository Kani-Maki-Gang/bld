use crate::config::BldConfig;
use crate::term::print_error;
use crate::types::{BldError, Result};
use actix::{Arbiter, System};
use actix_http::Payload;
use actix_web::client::Client;
use actix_web::dev::Decompress;
use actix_web::error::PayloadError;
use awc::http::StatusCode;
use awc::ClientResponse;
use bytes::Bytes;
use clap::ArgMatches;
use futures::Stream;
use std::pin::Pin;

type StdResult<T, V> = std::result::Result<T, V>;

type ServerResponse = ClientResponse<
    Decompress<Payload<Pin<Box<dyn Stream<Item = StdResult<Bytes, PayloadError>>>>>>,
>;

fn server_not_in_config() -> Result<()> {
    let message = String::from("server not found in config");
    Err(BldError::Other(message))
}

fn no_server_in_config() -> Result<()> {
    let message = String::from("no server found in config");
    Err(BldError::Other(message))
}

fn handle_body(body: &StdResult<Bytes, PayloadError>) {
    let res = match body {
        Ok(b) => String::from_utf8_lossy(&b).to_string(),
        Err(e) => e.to_string(),
    };
    println!("{}", res);
}

async fn handle_response(resp: &mut ServerResponse) {
    let body = resp.body().await;
    match resp.status() {
        StatusCode::OK => handle_body(&body),
        StatusCode::BAD_REQUEST => handle_body(&body),
        _ => println!("unexpected response from server"),
    }
}

fn exec_request(host: String, port: i64) {
    let system = System::new("bld-hist");
    Arbiter::spawn(async move {
        let url = format!("http://{}:{}/hist", host, port);
        let client = Client::default();
        let mut response = client.get(url).header("User-Agent", "Bld").send().await;
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


pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
    let (host, port) = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => (srv.host.to_string(), srv.port),
            None => return server_not_in_config(),
        },
        None => match servers.iter().next() {
            Some(srv) => (srv.host.to_string(), srv.port),
            None => return no_server_in_config(),
        },
    };
    exec_request(host, port);
    Ok(())
}
