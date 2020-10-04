mod client;
mod messages;

pub use client::*;
pub use messages::*;

use actix::{io::SinkWrite, Actor, StreamHandler};
use awc::Client;
use crate::config::BldConfig;
use crate::run::read;
use crate::run::socket::{PipelineWebSocketClient, RunPipelineMessage};
use futures::stream::StreamExt;
use std::io::{self, Error, ErrorKind};

pub async fn socket_client(server: &str, pipeline_name: &str) -> io::Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
    let server = match servers.iter().find(|s| s.name == server) {
        Some(server) => server,
        None => return Err(Error::new(ErrorKind::Other, "no server found")),
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

    let pipeline = read(pipeline_name)?;
    let _ = addr.send(RunPipelineMessage(pipeline)).await;

    Ok(())
}
