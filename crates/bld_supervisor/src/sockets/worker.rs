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

#[cfg(test)]
mod tests {
    use super::handle_message;
    use actix_web::web::Bytes;
    use bld_models::dtos::WorkerMessages;

    fn to_bytes(msg: &WorkerMessages) -> Bytes {
        Bytes::from(serde_json::to_vec(msg).unwrap())
    }

    #[tokio::test]
    async fn handle_message_ack_does_not_set_pid_and_is_not_completed() {
        let mut pid = None;
        let completed = handle_message(&to_bytes(&WorkerMessages::Ack), &mut pid)
            .await
            .unwrap();
        assert!(!completed);
        assert_eq!(pid, None);
    }

    #[tokio::test]
    async fn handle_message_who_am_i_sets_pid() {
        let mut pid = None;
        let completed = handle_message(&to_bytes(&WorkerMessages::WhoAmI { pid: 42 }), &mut pid)
            .await
            .unwrap();
        assert!(!completed);
        assert_eq!(pid, Some(42));
    }

    #[tokio::test]
    async fn handle_message_completed_signals_completion() {
        let mut pid = None;
        let completed = handle_message(&to_bytes(&WorkerMessages::Completed), &mut pid)
            .await
            .unwrap();
        assert!(completed);
    }

    #[tokio::test]
    async fn handle_message_invalid_payload_errors() {
        let mut pid = None;
        let result = handle_message(&Bytes::from_static(b"not json"), &mut pid).await;
        assert!(result.is_err());
    }
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
                            break;
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
