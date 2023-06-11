use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, Context, Handler, StreamHandler, System};
use actix_codec::Framed;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bld_core::messages::MonitInfo;
use futures::stream::SplitSink;
use tracing::debug;

pub struct MonitClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl MonitClient {
    pub fn new(writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>) -> Self {
        Self { writer }
    }
}

impl Actor for MonitClient {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("monit socket started");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("monit socket stoppped");
        if let Some(sys) = System::try_current() {
            sys.stop();
        }
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
