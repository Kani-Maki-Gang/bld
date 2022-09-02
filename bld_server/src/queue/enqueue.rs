use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, AsyncContext, Context, Handler, StreamHandler, System};
use actix_codec::Framed;
use actix_web::web::Bytes;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bld_supervisor::base::ServerMessages;
use futures::stream::SplitSink;
use std::time::Duration;
use tracing::{debug, info};

pub struct EnqueueClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl EnqueueClient {
    pub fn new(
        writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    ) -> Self {
        Self { writer }
    }

    fn heartbeat(&mut self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::new(1, 0), |act, ctx| {
            info!("sending heartbeat to supervisor");
            let _ = act.writer.write(Message::Ping(Bytes::from_static(b"")));
            act.heartbeat(ctx);
        });
    }
}

impl Actor for EnqueueClient {
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
