use crate::queues::WorkerQueueSender;
use actix_web::{
    HttpRequest, Responder,
    rt::{spawn, time},
    web::{self, Bytes, Data},
};
use actix_ws::{CloseCode, CloseReason, Message, handle};
use anyhow::Result;
use bld_models::dtos::WorkerMessages;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

const HEARTBEAT_INTERVAL_MS: u64 = 5_000;
const CLIENT_TIMEOUT_MS: u64 = 15_000;

async fn handle_message(
    bytes: &Bytes,
    worker_queue_tx: Data<WorkerQueueSender>,
    worker_pid: &mut Option<u32>,
) -> Result<bool> {
    let msg: WorkerMessages = serde_json::from_slice(&bytes[..])?;
    let completed = match msg {
        WorkerMessages::Ack => {
            info!("a new worker connection was acknowledged");
            false
        }
        WorkerMessages::WhoAmI { pid } => {
            info!("worker with pid: {pid} sent a whoami message");
            worker_pid.replace(pid);
            false
        }
        WorkerMessages::Completed => {
            info!("worker just completed, starting cleanup");
            if let Some(pid) = worker_pid {
                debug!("dequeue of worker with pid: {}", pid);
                worker_queue_tx.dequeue(*pid).await?;
            }
            true
        }
    };
    Ok(completed)
}

pub async fn ws(
    req: HttpRequest,
    body: web::Payload,
    worker_queue_tx: Data<WorkerQueueSender>,
) -> actix_web::Result<impl Responder> {
    let (response, mut session, mut msg_stream) = handle(&req, body)?;

    spawn(async move {
        let mut reason: Option<CloseReason> = None;
        let mut worker_pid: Option<u32> = None;
        let mut hb_interval = time::interval(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        let mut last_pong = Instant::now();

        'outer: loop {
            tokio::select! {
                msg = msg_stream.recv() => {
                    let Some(Ok(message)) = msg else { break; };
                    match message {
                        Message::Binary(bytes) => {
                            debug!("received binary message");
                            match handle_message(&bytes, worker_queue_tx.clone(), &mut worker_pid).await {
                                Ok(true) => break 'outer,
                                Ok(false) => {}
                                Err(e) => {
                                    reason = Some(CloseCode::Error.into());
                                    let _ = session
                                        .text("internal server error")
                                        .await
                                        .inspect_err(|e| error!("{e}"));
                                    error!("handling message error. {e}")
                                }
                            }
                        }

                        Message::Ping(msg) => {
                            if let Err(e) = session.pong(&msg).await {
                                reason = Some(CloseCode::Error.into());
                                error!("{e}");
                                break;
                            }
                        }

                        Message::Pong(_) => {
                            last_pong = Instant::now();
                        }

                        Message::Continuation(_) | Message::Nop => {}

                        Message::Close(r) => {
                            reason = r;
                            break;
                        }

                        _ => {
                            break;
                        }
                    }
                }

                _ = hb_interval.tick() => {
                    if Instant::now().duration_since(last_pong)
                        > Duration::from_millis(CLIENT_TIMEOUT_MS)
                    {
                        warn!("client heartbeat timed out, closing session");
                        reason = Some(CloseCode::Away.into());
                        break;
                    }
                    if let Err(e) = session.ping(b"").await {
                        error!("ping failed: {e}");
                        reason = Some(CloseCode::Error.into());
                        break;
                    }
                }
            }
        }

        if let Some(pid) = worker_pid {
            debug!("dequeue of worker with pid: {}", pid);
            let _ = worker_queue_tx
                .dequeue(pid)
                .await
                .inspect_err(|e| error!("{e}"));
        }

        if let Err(e) = session.close(reason).await {
            error!("encountered error while closing websocket session due to {e}");
        }
    });

    Ok(response)
}
