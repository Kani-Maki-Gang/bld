mod client;
mod messages;

pub use client::*;
pub use messages::*;

use crate::config::BldConfig;
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::headers;
use crate::helpers::term::print_error;
use crate::run::socket::{ExecutePipelineSocketClient, ExecutePipelineSocketMessage};
use crate::types::Result;
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use futures::stream::StreamExt;

async fn remote_invoke(name: String, server: String, _detach: bool) -> Result<()> {
    let config = BldConfig::load()?;
    let srv = config.remote.server(&server)?;
    let (srv_name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        },
        None => (&srv.name, &srv.auth),
    };
    let url = format!("http://{}:{}/ws-exec/", srv.host, srv.port);
    let headers = headers(srv_name, auth)?;
    let mut client = Client::new().ws(url);
    for (key, value) in headers.iter() {
        client = client.header(&key[..], &value[..]);
    }
    let (_, framed) = client.connect().await?;
    let (sink, stream) = framed.split();
    let addr = ExecutePipelineSocketClient::create(|ctx| {
        ExecutePipelineSocketClient::add_stream(stream, ctx);
        ExecutePipelineSocketClient::new(SinkWrite::new(sink, ctx))
    });
    let _ = addr.send(ExecutePipelineSocketMessage(name)).await;
    Ok(())
}

pub fn on_server(name: String, server: String, detach: bool) -> Result<()> {
    let system = System::new("bld");
    Arbiter::spawn(async move {
        if let Err(e) = remote_invoke(name, server, detach).await {
            let _ = print_error(&e.to_string());
            System::current().stop();
        }
    });
    let _ = system.run();
    Ok(())
}
