use crate::queues::WorkerQueueSender;
use actix_web::{
    HttpRequest, Responder,
    rt::spawn,
    web::{self, Bytes, Data},
};
use actix_ws::{CloseReason, Message, handle};
use anyhow::Result;
use bld_models::dtos::WorkerMessages;
use tracing::{debug, error, info};

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
        while let Some(Ok(msg)) = msg_stream.recv().await {
            match msg {
                Message::Binary(bytes) => {
                    debug!("received binary message");
                    match handle_message(&bytes, worker_queue_tx.clone(), &mut worker_pid).await {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => error!("handling message error. {e}"),
                    }
                }

                Message::Ping(msg) => {
                    if let Err(e) = session.pong(&msg).await {
                        error!("{e}");
                        break;
                    }
                }

                Message::Continuation(_) | Message::Pong(_) | Message::Nop => {}

                Message::Close(r) => {
                    reason = r;
                    break;
                }


                _ => {
                    break;
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
