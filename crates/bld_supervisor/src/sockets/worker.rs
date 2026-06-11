use crate::queues::WorkerQueueSender;
use actix_web::{
    HttpRequest, Responder,
    rt::spawn,
    web::{self, Bytes, Data},
};
use anyhow::Result;
use bld_models::dtos::WorkerMessages;
use bld_sock::session::{self, WebSocketMessage};
use tracing::{debug, error, info};

async fn handle_message(bytes: &Bytes, worker_pid: &mut Option<u32>) -> Result<bool> {
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
    let (response, mut handler) = session::handle(&req, body)?;

    spawn(async move {
        let mut worker_pid: Option<u32> = None;

        loop {
            match handler.next().await {
                WebSocketMessage::Binary(bytes) => {
                    debug!("received binary message");
                    match handle_message(&bytes, &mut worker_pid).await {
                        Ok(true) => break,
                        Ok(false) => {}
                        Err(e) => {
                            let session = handler.session();
                            let _ = session
                                .text("internal server error")
                                .await
                                .inspect_err(|e| error!("{e}"));
                            error!("handling message error. {e}");
                            handler.error();
                        }
                    }
                }
                WebSocketMessage::Continue => {}
                _ => break,
            }
        }

        if let Some(pid) = worker_pid {
            debug!("dequeue of worker with pid: {}", pid);
            let _ = worker_queue_tx
                .dequeue(pid)
                .await
                .inspect_err(|e| error!("{e}"));
        }

        handler.cleanup().await;
    });

    Ok(response)
}
