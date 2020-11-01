use crate::run::socket::RunPipelineMessage;
use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, AsyncContext, Context, Handler, StreamHandler, System};
use actix_codec::Framed;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bytes::Bytes;
use futures::stream::SplitSink;
use std::time::Duration;

pub struct PipelineWebSocketClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl PipelineWebSocketClient {
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

impl Actor for PipelineWebSocketClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.heartbeat(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        System::current().stop();
    }
}

impl Handler<RunPipelineMessage> for PipelineWebSocketClient {
    type Result = ();

    fn handle(&mut self, msg: RunPipelineMessage, _ctx: &mut Self::Context) {
        let _ = self.writer.write(Message::Text(msg.0));
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for PipelineWebSocketClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        match msg {
            Ok(Frame::Text(bt)) => println!("{}", String::from_utf8_lossy(&bt[..])),
            Ok(Frame::Close(_)) => System::current().stop(),
            _ => {}
        }
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for PipelineWebSocketClient {}
