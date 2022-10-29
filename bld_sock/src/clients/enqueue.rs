use crate::messages::ServerMessages;
use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, Context, Handler, StreamHandler};
use actix_codec::Framed;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use futures::stream::SplitSink;
use tracing::{debug, info};

pub struct EnqueueClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl EnqueueClient {
    pub fn new(writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>) -> Self {
        Self { writer }
    }
}

impl Actor for EnqueueClient {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("supervisor socket started");
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        debug!("supervisor socket stopped");
    }
}

impl Handler<ServerMessages> for EnqueueClient {
    type Result = ();

    fn handle(&mut self, msg: ServerMessages, _ctx: &mut Self::Context) {
        if let Ok(bytes) = serde_json::to_vec(&msg) {
            let _ = self.writer.write(Message::Binary(bytes.into()));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for EnqueueClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Context<Self>) {
        match msg {
            Ok(Frame::Text(bt)) => println!("{}", String::from_utf8_lossy(&bt)),
            Ok(Frame::Close(_)) => {
                info!("web socket connection stopped due to a sent closed frame");
                ctx.stop();
            }
            _ => {}
        }
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        info!("web socket communication finished");
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for EnqueueClient {}
