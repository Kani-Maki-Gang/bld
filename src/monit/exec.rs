use crate::config::BldConfig;
use crate::definitions::TOOL_DEFAULT_PIPELINE;
use crate::monit::{MonitorPipelineSocketClient, MonitorPipelineSocketMessage};
use crate::term::print_error;
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use clap::ArgMatches;
use futures::stream::StreamExt;
use std::io::{self, Error, ErrorKind};

async fn remote_invoke(host: String, port: i64, id: String) -> io::Result<()> {
    let url = format!("http://{}:{}/ws-monit", host, port);
    let (_, framed) = Client::new()
        .ws(url)
        .connect()
        .await
        .map_err(|e| println!("Error: {}", e))
        .unwrap();
    let (sink, stream) = framed.split();
    let addr = MonitorPipelineSocketClient::create(|ctx| {
        MonitorPipelineSocketClient::add_stream(stream, ctx);
        MonitorPipelineSocketClient::new(SinkWrite::new(sink, ctx))
    });
    let _ = addr.send(MonitorPipelineSocketMessage(id)).await;
    Ok(())
}

fn exec_request(host: String, port: i64, id: String) {
    let system = System::new("bld-monit");

    Arbiter::spawn(async move {
        if let Err(e) = remote_invoke(host, port, id).await {
            let _ = print_error(&e.to_string());
            System::current().stop();
        }
    });

    let _ = system.run();
}

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;

    let id = match matches.value_of("pipeline-id") {
        Some(pipeline) => pipeline,
        None => TOOL_DEFAULT_PIPELINE,
    };

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

    exec_request(host.to_string(), port, id.to_string());
    Ok(())
}
