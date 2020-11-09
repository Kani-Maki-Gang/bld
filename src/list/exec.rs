use crate::config::BldConfig;
use actix::{Arbiter, System};
use actix_http::Payload;
use actix_web::{client::Client, dev::Decompress, error::PayloadError};
use awc::{http::StatusCode, ClientResponse};
use bytes::Bytes;
use clap::ArgMatches;
use futures::Stream;
use std::io::{self, Error, ErrorKind};
use std::pin::Pin;

type ServerResponse =
    ClientResponse<Decompress<Payload<Pin<Box<dyn Stream<Item = Result<Bytes, PayloadError>>>>>>>;

fn handle_body(body: &Result<Bytes, PayloadError>) {
    match body {
        Ok(b) => println!("{}", String::from_utf8_lossy(&b)),
        Err(e) => println!("{}", e.to_string()),
    }
}

async fn handle_response(resp: &mut ServerResponse) {
    let body = resp.body().await;
    match resp.status() {
        StatusCode::OK => handle_body(&body),
        StatusCode::BAD_REQUEST => handle_body(&body),
        _ => println!("unexpected response from server"),
    }
}

fn exec_request(host: String, port: i64, _running: bool) {
    let system = System::new("bld-ls");

    Arbiter::spawn(async move {
        let url = format!("http://{}:{}/list", host, port);
        let client = Client::default();
        let mut response = client
            .get(url)
            .header("User-Agent", "Bld")
            .no_decompress()
            .send()
            .await;
        match response.as_mut() {
            Ok(resp) => handle_response(resp).await,
            Err(e) => println!("{}", e.to_string()),
        }
        System::current().stop();
    });

    let _ = system.run();
}

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;

    let (host, port) = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => (&srv.host, srv.port),
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "server not found in configuration",
                ))
            }
        },
        None => match servers.iter().next() {
            Some(srv) => (&srv.host, srv.port),
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "no server found in configuration",
                ))
            }
        },
    };

    let running = matches.is_present("running");

    exec_request(host.to_string(), port, running);
    Ok(())
}
