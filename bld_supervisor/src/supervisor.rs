use crate::queues::WorkerQueue;
use crate::sockets::{ws_server_socket, ws_worker_socket};
use actix_web::web::{get, resource, Data};
use actix_web::{App, HttpServer};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_core::database::new_connection_pool;
use std::sync::Mutex;

pub async fn start(config: BldConfig) -> Result<()> {
    let address = format!(
        "{}:{}",
        config.local.supervisor.host, config.local.supervisor.port
    );
    let config = Data::new(config);
    let config_clone = config.clone();
    let pool = Data::new(new_connection_pool(&config.local.db)?);
    let worker_queue = Data::new(Mutex::new(WorkerQueue::new(
        config.local.supervisor.workers.try_into()?,
        config.clone(),
        pool.clone(),
    )));

    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(config_clone.clone())
            .app_data(pool.clone())
            .app_data(worker_queue.clone())
            .service(resource("/ws-server/").route(get().to(ws_server_socket)))
            .service(resource("/ws-worker/").route(get().to(ws_worker_socket)))
    });

    server = match &config.local.supervisor.tls {
        Some(tls) => {
            let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
            builder.set_private_key_file(&tls.private_key, SslFiletype::PEM)?;
            builder.set_certificate_chain_file(&tls.cert_chain)?;
            server.bind_openssl(address, builder)?
        }
        None => server.bind(address)?
    };

    server.run().await.map_err(|e| anyhow!(e))
}
