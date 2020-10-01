use actix::prelude::*;
use actix_web_actors::ws;

pub struct RunPipelineWS;

impl Actor for RunPipelineWS {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RunPipelineWS {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                println!("{}", &text);
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            },
            Err(_) => ctx.stop(),
            _ => {},
        }
    }
}
