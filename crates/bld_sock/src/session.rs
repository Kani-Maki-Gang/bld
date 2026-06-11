use std::time::Duration;

use actix_web::{HttpRequest, HttpResponse, Result, web::Payload};
use actix_ws::{CloseCode, CloseReason, Message, MessageStream, Session};
use bytes::Bytes;
use bytestring::ByteString;
use futures::StreamExt;
use tokio::time::{self, Instant, Interval, MissedTickBehavior};
use tracing::{debug, error, warn};

const HEARTBEAT_INTERVAL_MS: u64 = 5_000;
const CLIENT_TIMEOUT_MS: u64 = 15_000;

pub enum WebSocketMessage {
    Text(ByteString),
    Binary(Bytes),
    Continue,
    Completed,
}

pub struct WebSocketHandler {
    session: Session,
    stream: MessageStream,
    term_reason: Option<CloseReason>,
    last_pong: Instant,
    hb_interval: Interval,
}

impl WebSocketHandler {
    pub fn new(session: Session, stream: MessageStream) -> Self {
        let mut hb_interval = time::interval(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        hb_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        Self {
            session,
            stream,
            term_reason: None,
            last_pong: Instant::now(),
            hb_interval,
        }
    }

    pub fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    pub fn error(&mut self) {
        self.term_reason = Some(CloseCode::Error.into());
    }

    pub async fn next(&mut self) -> WebSocketMessage {
        tokio::select! {
            biased;

            Some(msg) = self.stream.next() => {
                let Ok(message) = msg.inspect_err(|e| error!("{e}")) else {
                    self.error();
                    return WebSocketMessage::Completed;
                };
                // any inbound frame proves the peer is alive
                self.last_pong = Instant::now();
                match message {
                    Message::Text(txt) => WebSocketMessage::Text(txt),

                    Message::Binary(data) => WebSocketMessage::Binary(data),

                    Message::Ping(msg) => {
                        if let Err(e) = self.session.pong(&msg).await {
                            error!("{e}");
                            self.error();
                            WebSocketMessage::Completed
                        } else {
                            WebSocketMessage::Continue
                        }
                    }

                    Message::Pong(_) => WebSocketMessage::Continue,

                    Message::Continuation(_) | Message::Nop => WebSocketMessage::Continue,

                    Message::Close(reason) => {
                        self.term_reason = reason;
                        WebSocketMessage::Completed
                    }
                }
            }

            _ = self.hb_interval.tick() => {
                if Instant::now().duration_since(self.last_pong)
                    > Duration::from_millis(CLIENT_TIMEOUT_MS)
                {
                    self.error();
                    warn!("client heartbeat timed out, closing session");
                    return WebSocketMessage::Completed;
                }
                if let Err(e) = self.session.ping(b"").await {
                    self.error();
                    error!("ping failed: {e}");
                    return WebSocketMessage::Completed;
                }
                WebSocketMessage::Continue
            }
        }
    }

    pub async fn cleanup(self) {
        let _ = self
            .session
            .close(self.term_reason)
            .await
            .inspect(|_| debug!("closed session successfully"))
            .inspect_err(|e| {
                error!("encountered error while closing websocket session due to {e}")
            });
    }
}

pub fn handle(req: &HttpRequest, body: Payload) -> Result<(HttpResponse, WebSocketHandler)> {
    let (response, session, msg_stream) = actix_ws::handle(req, body)?;
    Ok((response, WebSocketHandler::new(session, msg_stream)))
}
