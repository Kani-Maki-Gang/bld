use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, AsyncContext, Context, Handler, StreamHandler, System};
use actix_codec::Framed;
use actix_web::web::Bytes;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bld_supervisor::base::WorkerMessages;
use futures::stream::SplitSink;
use std::time::Duration;
use tracing::debug;

pub struct WorkerClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl WorkerClient {
    pub fn new(
        writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    ) -> Self {
        Self { writer }
    }

    fn heartbeat(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::from_millis(500), |act, ctx| {
            let _ = act.writer.write(Message::Ping(Bytes::from_static(b"")));
            act.heartbeat(ctx);
        });
    }
}

impl Actor for WorkerClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("supervisor socket started");
        self.heartbeat(ctx)
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        debug!("supervisor socket stopped");
        System::current().stop();
    }
}

impl Handler<WorkerMessages> for WorkerClient {
    type Result = ();

    fn handle(&mut self, msg: WorkerMessages, _ctx: &mut Self::Context) {
        if let Ok(bytes) = serde_json::to_vec(&msg) {
            let _ = self.writer.write(Message::Binary(bytes.into()));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for WorkerClient {
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

impl WriteHandler<WsProtocolError> for WorkerClient {}
