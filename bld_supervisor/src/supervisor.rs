use crate::queues::WorkerQueue;
use crate::sockets::{ws_active_worker, ws_queue_worker};
use actix_web::{
    web::{get, resource, Data},
    App, HttpServer,
};
use anyhow::anyhow;
use bld_config::BldConfig;
use bld_core::database::new_connection_pool;
use std::sync::Mutex;

pub async fn start(config: BldConfig) -> anyhow::Result<()> {
    let url = format!(
        "{}:{}",
        config.local.supervisor.host, config.local.supervisor.port
    );
    let pool = Data::new(new_connection_pool(&config.local.db)?);
    let worker_queue = Data::new(Mutex::new(WorkerQueue::new(
        config.local.supervisor.workers.try_into()?,
        pool.clone(),
    )));
    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(worker_queue.clone())
            .service(resource("/ws-queue/").route(get().to(ws_queue_worker)))
            .service(resource("/ws-worker/").route(get().to(ws_active_worker)))
    })
    .bind(url)?
    .run()
    .await
    .map_err(|e| anyhow!(e))
}
