use crate::socket::ExecInfo;
use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, Context, Handler, StreamHandler};
use actix_codec::Framed;
use actix_web::rt::System;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use futures::stream::SplitSink;
use tracing::debug;

pub struct ExecClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl ExecClient {
    pub fn new(writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>) -> Self {
        Self { writer }
    }
}

impl Actor for ExecClient {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("exec socket started");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("exec socket stopped");
        if let Some(current) = System::try_current() {
            current.stop();
        }
    }
}

impl Handler<ExecInfo> for ExecClient {
    type Result = ();

    fn handle(&mut self, msg: ExecInfo, _ctx: &mut Self::Context) {
        if let Ok(msg) = serde_json::to_string(&msg) {
            let _ = self.writer.write(Message::Text(msg.into()));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for ExecClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Context<Self>) {
        match msg {
            Ok(Frame::Text(bt)) => println!("{}", String::from_utf8_lossy(&bt[..])),
            Ok(Frame::Close(_)) => ctx.stop(),
            _ => {}
        }
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for ExecClient {}
