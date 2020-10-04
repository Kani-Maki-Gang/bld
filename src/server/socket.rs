use actix::prelude::*;
use actix_web_actors::ws;
use crate::run;
use crate::term;

pub struct PipelineWebSocketServer;

impl PipelineWebSocketServer {
    pub fn new() -> Self {
        Self { }
    }
}

impl Actor for PipelineWebSocketServer {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for PipelineWebSocketServer {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        // process websocket messages
        match msg {
            Ok(ws::Message::Text(text)) => {
                let _ = futures::executor::block_on(sync_wrapper(&text));
                ctx.text(text);
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

async fn sync_wrapper(text: &str) {
    if let Err(e) = run::sync(String::from(text)).await.await {
        println!("{}", e.to_string());
    }
}
