mod client;
mod messages;

pub use client::*;
pub use messages::*;

use crate::config::BldConfig;
use crate::run::Pipeline;
use crate::run::socket::{PipelineWebSocketClient, RunPipelineMessage};
use crate::term::print_error;
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use futures::stream::StreamExt;
use serde_json::json;
use std::io::{self, Error, ErrorKind};

async fn remote_invoke(name: String, server: String) -> io::Result<()> {
    let config = BldConfig::load()?;
    let server = config
        .remote
        .servers
        .iter()
        .find(|s| s.name == server);
    let server = match server {
        Some(s) => s,
        None => return Err(Error::new(ErrorKind::Other, "server not found in config"))
    };

    let url = format!("http://{}:{}/ws/", server.host, server.port);
    let (_, framed) = Client::new()
        .ws(url)
        .connect()
        .await
        .map_err(|e| println!("Error: {}", e))
        .unwrap();

    let (sink, stream) = framed.split();
    let addr = PipelineWebSocketClient::create(|ctx| {
        PipelineWebSocketClient::add_stream(stream, ctx);
        PipelineWebSocketClient::new(SinkWrite::new(sink, ctx))
    });

    let message = json!({
        "name": name,
        "pipeline": Pipeline::read(&name)?
    }).to_string();
    let _ = addr.send(RunPipelineMessage(message)).await;

    Ok(())
}

pub fn on_server(name: String, server: String) -> io::Result<()> {
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
