use actix::prelude::*;
use actix::fut::WrapFuture;
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
        match msg {
            Ok(ws::Message::Text(text)) => {
                let content = String::from(&text);
                let run_fut = run(content).into_actor(self);
                ctx.wait(run_fut);
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

async fn run(text: String) {
    if let Err(e) = run::from_src(text).await.await {
        let _ = term::print_error(&format!("{}", e));
    }
}
