mod client;
mod messages;

pub use client::*;
pub use messages::*;

use crate::config::BldConfig;
use crate::run::socket::{ExecutePipelineSocketClient, ExecutePipelineSocketMessage};
use crate::term::print_error;
use crate::types::{BldError, Result};
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use futures::stream::StreamExt;

fn server_not_found() -> Result<()> {
    let message = String::from("server not found in config");
    Err(BldError::Other(message))
}

async fn remote_invoke(name: String, server: String) -> Result<()> {
    let config = BldConfig::load()?;
    let server = config.remote.servers.iter().find(|s| s.name == server);
    let server = match server {
        Some(s) => s,
        None => return server_not_found(),
    };
    let url = format!("http://{}:{}/ws-exec/", server.host, server.port);
    let (_, framed) = Client::new().ws(url).connect().await?;
    let (sink, stream) = framed.split();
    let addr = ExecutePipelineSocketClient::create(|ctx| {
        ExecutePipelineSocketClient::add_stream(stream, ctx);
        ExecutePipelineSocketClient::new(SinkWrite::new(sink, ctx))
    });
    let _ = addr.send(ExecutePipelineSocketMessage(name)).await;
    Ok(())
}

pub fn on_server(name: String, server: String) -> Result<()> {
    let system = System::new("bld");
    Arbiter::spawn(async move {
        if let Err(e) = remote_invoke(name, server).await {
            let _ = print_error(&e.to_string());
            System::current().stop();
        }
    });
    let _ = system.run();
    Ok(())
}
