use crate::config::{definitions::TOOL_DEFAULT_PIPELINE, BldConfig};
use crate::helpers::errors::{no_server_in_config, server_not_in_config};
use crate::helpers::term::print_error;
use crate::monit::{MonitorPipelineSocketClient, MonitorPipelineSocketMessage};
use crate::types::Result;
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use clap::ArgMatches;
use futures::stream::StreamExt;

async fn remote_invoke(host: String, port: i64, id: String) -> Result<()> {
    let url = format!("http://{}:{}/ws-monit", host, port);
    let (_, framed) = Client::new().ws(url).connect().await?;
    let (sink, stream) = framed.split();
    let addr = MonitorPipelineSocketClient::create(|ctx| {
        MonitorPipelineSocketClient::add_stream(stream, ctx);
        MonitorPipelineSocketClient::new(SinkWrite::new(sink, ctx))
    });
    addr.send(MonitorPipelineSocketMessage(id)).await?;
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

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
    let id = matches
        .value_of("pipeline-id")
        .or(Some(TOOL_DEFAULT_PIPELINE))
        .unwrap();
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
    exec_request(host.to_string(), port, id.to_string());
    Ok(())
}
