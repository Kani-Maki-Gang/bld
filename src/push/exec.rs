use crate::config::BldConfig;
use crate::definitions::TOOL_DEFAULT_PIPELINE;
use crate::persist::NullLogger;
use crate::run::Pipeline;
use crate::term::print_error;
use crate::types::{BldError, PushInfo, Result};
use actix::{Arbiter, System};
use actix_http::Payload;
use actix_web::client::Client;
use actix_web::dev::Decompress;
use actix_web::error::PayloadError;
use awc::http::StatusCode;
use awc::ClientResponse;
use bytes::Bytes;
use clap::ArgMatches;
use futures::{Future, Stream};
use std::collections::HashSet;
use std::pin::Pin;

type StdResult<T, V> = std::result::Result<T, V>;
type RecursiveFuture = Pin<Box<dyn Future<Output = Result<HashSet<(String, String)>>>>>;
type ServerResponse = ClientResponse<
    Decompress<Payload<Pin<Box<dyn Stream<Item = StdResult<Bytes, PayloadError>>>>>>,
>;

fn server_not_in_config() -> Result<()> {
    Err(BldError::Other("server not found in config".to_string()))
}

fn no_server_in_config() -> Result<()> {
    Err(BldError::Other("no server in config".to_string()))
}

async fn build_payload(name: String) -> RecursiveFuture {
    Box::pin(async move {
        let src = Pipeline::read(&name)?;
        let pipeline = Pipeline::parse(&src, NullLogger::atom()).await?;
        let mut set = HashSet::new();
        set.insert((name.to_string(), src));

        for step in pipeline.steps.iter() {
            if let Some(pipeline) = &step.call {
                let subset = build_payload(pipeline.to_string()).await.await?;
                for entry in subset {
                    set.insert(entry);
                }
            }
        }

        Ok(set)
    })
}

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
        _ => String::from("unexpected response from server"),
    };
    if res.len() > 0 {
        println!("{}", res);
    } else {
        println!("Done.");
    }
}

fn exec_request(host: String, port: i64, name: String) {
    let system = System::new("bld-push");

    Arbiter::spawn(async move {
        match build_payload(name).await.await {
            Ok(payload) => {
                let data: Vec<PushInfo> = payload
                    .iter()
                    .map(|(n, s)| {
                        println!("Pushing {}...", n);
                        PushInfo::new(n, s)
                    })
                    .collect();
                let url = format!("http://{}:{}/push", host, port);
                let client = Client::default();
                let mut response = client
                    .post(url)
                    .header("User-Agent", "Bld")
                    .send_json(&data)
                    .await;
                match response.as_mut() {
                    Ok(r) => handle_response(r).await,
                    Err(e) => {
                        let _ = print_error(&e.to_string());
                    }
                }
            }
            Err(e) => eprintln!("{}", e.to_string()),
        }
        System::current().stop();
    });

    let _ = system.run();
}

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
    let name = match matches.value_of("pipeline") {
        Some(pipeline) => pipeline.to_string(),
        None => TOOL_DEFAULT_PIPELINE.to_string(),
    };
    let (host, port) = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => (&srv.host, srv.port),
            None => return server_not_in_config(),
        },
        None => match servers.iter().next() {
            Some(srv) => (&srv.host, srv.port),
            None => return no_server_in_config(),
        },
    };
    exec_request(host.to_string(), port, name);
    Ok(())
}
