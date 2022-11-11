use crate::messages::RunInfo;
use actix::io::{SinkWrite, WriteHandler};
use actix::{Actor, ActorContext, Context, Handler, StreamHandler};
use actix_codec::Framed;
use actix_web::rt::{spawn, System};
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use awc::BoxedSocket;
use bld_core::logger::LoggerSender;
use futures::stream::SplitSink;
use std::sync::Arc;
use tracing::{debug, error};

pub struct ExecClient {
    logger: Arc<LoggerSender>,
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl ExecClient {
    pub fn new(
        logger: Arc<LoggerSender>,
        writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
    ) -> Self {
        Self { logger, writer }
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

impl Handler<RunInfo> for ExecClient {
    type Result = ();

    fn handle(&mut self, msg: RunInfo, _ctx: &mut Self::Context) {
        if let Ok(msg) = serde_json::to_string(&msg) {
            let _ = self.writer.write(Message::Text(msg.into()));
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for ExecClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, ctx: &mut Context<Self>) {
        match msg {
            Ok(Frame::Text(bt)) => {
                let message = format!("{}", String::from_utf8_lossy(&bt[..]));
                let logger = self.logger.clone();
                spawn(async move {
                    if let Err(e) = logger.write_line(message).await {
                        error!("{e}");
                    }
                });
            }
            Ok(Frame::Close(_)) => ctx.stop(),
            _ => {}
        }
    }

    fn finished(&mut self, ctx: &mut Context<Self>) {
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for ExecClient {}
