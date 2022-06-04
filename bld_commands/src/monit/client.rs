use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, AsyncContext, Context, Handler, StreamHandler, System};
use actix_codec::Framed;
use actix_web::web::Bytes;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bld_server::requests::MonitInfo;
use futures::stream::SplitSink;
use std::time::Duration;

pub struct MonitClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl MonitClient {
    pub fn new(writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>) -> Self {
        Self { writer }
    }

    fn heartbeat(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::new(1, 0), |act, ctx| {
            let _ = act.writer.write(Message::Ping(Bytes::from_static(b"")));
            act.heartbeat(ctx);
        });
    }
}

impl Actor for MonitClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.heartbeat(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        System::current().stop();
    }
}

impl Handler<MonitInfo> for MonitClient {
    type Result = ();

    fn handle(&mut self, msg: MonitInfo, _ctx: &mut Self::Context) {
        if let Ok(text) = serde_json::to_string(&msg) {
            let _ = self.writer.write(Message::Text(text.into()));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for MonitClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Context<Self>) {
        match msg {
            Ok(Frame::Text(bt)) => println!("{}", String::from_utf8_lossy(&bt)),
            Ok(Frame::Close(_)) => ctx.stop(),
            _ => {}
        }
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for MonitClient {}
