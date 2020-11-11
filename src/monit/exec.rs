use crate::config::BldConfig;
use crate::definitions::TOOL_DEFAULT_PIPELINE;
use crate::helpers::err;
use crate::monit::{MonitorPipelineSocketClient, MonitorPipelineSocketMessage};
use crate::term::print_error;
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use clap::ArgMatches;
use futures::stream::StreamExt;
use std::io;

async fn remote_invoke(host: String, port: i64, id: String) -> io::Result<()> {
    let url = format!("http://{}:{}/ws-monit", host, port);
    let (_, framed) = match Client::new().ws(url).connect().await {
        Ok(data) => data,
        Err(e) => return err(e.to_string()),
    };
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
            None => return err("server not found in config".to_string()),
        },
        None => match servers.iter().next() {
            Some(srv) => (&srv.host, srv.port),
            None => return err("no server found in config".to_string()),
        },
    };

    exec_request(host.to_string(), port, id.to_string());
    Ok(())
}
