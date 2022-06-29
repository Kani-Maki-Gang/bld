#![allow(unused_imports)]

pub mod data;
pub mod endpoints;
pub mod extractors;
pub mod requests;
pub mod responses;
pub mod sockets;

use crate::data::PipelineWorker;
use crate::endpoints::{
    auth_redirect, deps, ha_append_entries, ha_install_snapshot, ha_vote, hist, home, inspect,
    list, pull, push, remove, stop,
};
use crate::sockets::{ws_exec, ws_high_avail, ws_monit};
use actix::System;
use actix_web::{middleware, web, App, HttpServer};
use bld_config::BldConfig;
use bld_core::database::new_connection_pool;
use bld_core::high_avail::HighAvail;
use bld_core::proxies::ServerPipelineProxy;
use std::env::set_var;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info};

async fn set_worker_cleanup(workers: Arc<Mutex<Vec<PipelineWorker>>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        if let Ok(mut workers) = workers.lock() {
            debug!("starting cleanup of zombie workers");
            let mut zombies = vec![];
            for (i, worker) in workers.iter_mut().enumerate() {
                if worker.completed() {
                    let _ = worker.cleanup();
                    zombies.push(i);
                }
            }
            zombies.sort_by(|a, b| b.cmp(a));
            debug!(
                "cleaned up {} zombies removing them from lookup",
                zombies.len()
            );
            for i in zombies {
                workers.remove(i);
            }
        }
    }
}

pub async fn start(config: BldConfig, host: &str, port: i64) -> anyhow::Result<()> {
    info!("starting bld server at {}:{}", host, port);
    let workers = web::Data::new(Mutex::new(vec![]));
    let pool = new_connection_pool(&config.local.db)?;
    let ha = web::Data::new(HighAvail::new(&config, pool.clone()).await?);
    let pool = web::Data::new(pool);
    let cfg = web::Data::new(config);
    let prx = web::Data::new(ServerPipelineProxy::new(
        Arc::clone(&cfg),
        Arc::clone(&pool),
    ));
    System::current()
        .arbiter()
        .spawn(set_worker_cleanup(Arc::clone(&workers)));
    set_var("RUST_LOG", "actix_server=info,actix_web=debug");
    HttpServer::new(move || {
        App::new()
            .app_data(workers.clone())
            .app_data(cfg.clone())
            .app_data(ha.clone())
            .app_data(pool.clone())
            .app_data(prx.clone())
            .wrap(middleware::Logger::default())
            .service(ha_append_entries)
            .service(ha_install_snapshot)
            .service(ha_vote)
            .service(home)
            .service(auth_redirect)
            .service(hist)
            .service(list)
            .service(remove)
            .service(push)
            .service(deps)
            .service(pull)
            .service(stop)
            .service(inspect)
            .service(web::resource("/ws-exec/").route(web::get().to(ws_exec)))
            .service(web::resource("/ws-monit/").route(web::get().to(ws_monit)))
            .service(web::resource("/ws-ha/").route(web::get().to(ws_high_avail)))
    })
    .bind(format!("{host}:{port}"))?
    .run()
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
