use actix::io::{WriteHandler, SinkWrite};
use actix::{Actor, Context, Handler, StreamHandler};
use actix_codec::Framed;
use awc::BoxedSocket;
use awc::error::WsProtocolError;
use awc::ws::{Codec, Frame, Message};
use crate::run::socket::RunPipelineMessage;
use futures::stream::SplitSink;

pub struct PipelineWebSocketClient {
    writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>,
}

impl PipelineWebSocketClient {
    pub fn new(writer: SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>) -> Self {
        Self { writer }
    }
}

impl Actor for PipelineWebSocketClient {
    type Context = Context<Self>;
}

impl Handler<RunPipelineMessage> for PipelineWebSocketClient {
    type Result = ();

    fn handle(&mut self, msg: RunPipelineMessage, _ctx: &mut Self::Context) {
        match self.writer.write(Message::Text(msg.0)) {
            _ => {}
        }
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for PipelineWebSocketClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Context<Self>) {
        if let Ok(Frame::Text(txt)) = msg {
            println!("{:?}", txt);
        }
    }
}

impl WriteHandler<WsProtocolError> for PipelineWebSocketClient {}
