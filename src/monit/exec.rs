use crate::config::{definitions::TOOL_DEFAULT_PIPELINE, BldConfig};
use crate::helpers::errors::{auth_for_server_invalid, no_server_in_config, server_not_in_config};
use crate::helpers::request::headers;
use crate::helpers::term::print_error;
use crate::monit::{MonitorPipelineSocketClient, MonitorPipelineSocketMessage};
use crate::types::Result;
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use clap::ArgMatches;
use futures::stream::StreamExt;
use std::collections::HashMap;

struct MonitorConnectionInfo {
    host: String,
    port: i64,
    headers: HashMap<String, String>,
    id: String,
}

async fn remote_invoke(info: MonitorConnectionInfo) -> Result<()> {
    let url = format!("http://{}:{}/ws-monit", info.host, info.port);
    let mut client = Client::new().ws(url);
    for (key, value) in info.headers.iter() {
        client = client.header(&key[..], &value[..]);
    }
    let (_, framed) = client.connect().await?;
    let (sink, stream) = framed.split();
    let addr = MonitorPipelineSocketClient::create(|ctx| {
        MonitorPipelineSocketClient::add_stream(stream, ctx);
        MonitorPipelineSocketClient::new(SinkWrite::new(sink, ctx))
    });
    addr.send(MonitorPipelineSocketMessage(info.id)).await?;
    Ok(())
}

fn exec_request(info: MonitorConnectionInfo) {
    let system = System::new("bld-monit");
    Arbiter::spawn(async move {
        if let Err(e) = remote_invoke(info).await {
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
        .unwrap()
        .to_string();
    let srv = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => srv,
            None => return server_not_in_config(),
        },
        None => match servers.iter().next() {
            Some(srv) => srv,
            None => return no_server_in_config(),
        },
    };
    let (name, auth) = match &srv.same_auth_as {
        Some(name) => match servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        },
        None => (&srv.name, &srv.auth),
    };
    exec_request(MonitorConnectionInfo {
        host: srv.host.to_string(),
        port: srv.port,
        headers: headers(name, auth)?,
        id,
    });
    Ok(())
}
