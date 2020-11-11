use crate::config::BldConfig;
use crate::helpers::err;
use crate::term::print_error;
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
use std::io;
use std::pin::Pin;

type ServerResponse =
    ClientResponse<Decompress<Payload<Pin<Box<dyn Stream<Item = Result<Bytes, PayloadError>>>>>>>;

fn handle_body(body: &Result<Bytes, PayloadError>) {
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

fn exec_request(host: String, port: i64, _running: bool) {
    let system = System::new("bld-ls");

    Arbiter::spawn(async move {
        let url = format!("http://{}:{}/list", host, port);
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

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;

    let (host, port) = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => (&srv.host, srv.port),
            None => return err("server not found in config".to_string()),
        },
        None => match servers.iter().next() {
            Some(srv) => (&srv.host, srv.port),
            None => return err("no server found in config".to_string()),
        },
    };

    let running = matches.is_present("running");

    exec_request(host.to_string(), port, running);
    Ok(())
}
