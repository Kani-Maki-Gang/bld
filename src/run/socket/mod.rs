mod client;
mod messages;

pub use client::*;
pub use messages::*;

use crate::config::BldConfig;
use crate::run::read;
use crate::run::socket::{PipelineWebSocketClient, RunPipelineMessage};
use actix::{io::SinkWrite, Actor, Arbiter, StreamHandler, System};
use awc::Client;
use futures::stream::StreamExt;
use std::io;

pub fn on_server(server: String, pipeline_name: String) -> io::Result<()> {
    let system = System::new("bld");

    Arbiter::spawn(async move {
        if let Ok(config) = BldConfig::load() {
            let servers = config.remote.servers;

            if let Some(server) = servers.iter().find(|s| s.name == server) {
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

                if let Ok(pipeline) = read(&pipeline_name) {
                    let _ = addr.send(RunPipelineMessage(pipeline)).await;
                }
            }
        }
    });

    let _ = system.run();

    Ok(())
}
