use crate::monit::MonitorPipelineSocketMessage;
use crate::types::MonitInfo;
use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, AsyncContext, Context, Handler, StreamHandler, System};
use actix_codec::Framed;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bytes::Bytes;
use futures::stream::SplitSink;
use std::time::Duration;

pub struct MonitorPipelineSocketClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl MonitorPipelineSocketClient {
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

impl Actor for MonitorPipelineSocketClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.heartbeat(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        System::current().stop();
    }
}

impl Handler<MonitorPipelineSocketMessage> for MonitorPipelineSocketClient {
    type Result = ();

    fn handle(&mut self, msg: MonitorPipelineSocketMessage, _ctx: &mut Self::Context) {
        if let Ok(text) = serde_json::to_string(&MonitInfo::new(msg.0, msg.1)) {
            let _ = self.writer.write(Message::Text(text));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for MonitorPipelineSocketClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        match msg {
            Ok(Frame::Text(bt)) => println!("{}", String::from_utf8_lossy(&bt)),
            Ok(Frame::Close(_)) => System::current().stop(),
            _ => {}
        }
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for MonitorPipelineSocketClient {}
