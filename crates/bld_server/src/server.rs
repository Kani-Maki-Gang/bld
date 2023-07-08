use crate::cron::CronScheduler;
use crate::endpoints::{
    auth, check, cron, deps, hist, home, list, print, pull, push, remove, run, stop,
};
use crate::sockets::{exec, login, monit};
use crate::supervisor::channel::SupervisorMessageSender;
use actix_web::web::{get, resource};
use actix_web::{middleware, App, HttpServer};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::auth::LoginProcess;
use bld_core::database::new_connection_pool;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::sync::IntoData;
use bld_utils::tls::{load_server_certificate, load_server_private_key};
use rustls::ServerConfig;
use std::env::set_var;
use std::sync::Arc;
use tracing::info;

pub async fn start(config: BldConfig, host: String, port: i64) -> Result<()> {
    info!("starting bld server at {}:{}", host, port);

    let config = config.into_data();
    let client = config.openid_core_client().await?.into_data();
    let config_clone = config.clone();
    let pool = new_connection_pool(&config.local.db)?;
    let supervisor_sender = SupervisorMessageSender::new(Arc::clone(&config)).into_data();
    let logins = LoginProcess::new().into_data();
    let pool = pool.into_data();
    let prx = PipelineFileSystemProxy::server(Arc::clone(&config), Arc::clone(&pool)).into_data();
    let cron = CronScheduler::new(
        Arc::clone(&prx),
        Arc::clone(&pool),
        Arc::clone(&supervisor_sender),
    )
    .await?
    .into_data();

    set_var("RUST_LOG", "actix_server=info,actix_web=debug");
    let mut server = HttpServer::new(move || {
        App::new()
            .app_data(config_clone.clone())
            .app_data(client.clone())
            .app_data(supervisor_sender.clone())
            .app_data(logins.clone())
            .app_data(pool.clone())
            .app_data(prx.clone())
            .app_data(cron.clone())
            .wrap(middleware::Logger::default())
            .service(home::get)
            .service(auth::redirect)
            .service(auth::refresh)
            .service(check::get)
            .service(hist::get)
            .service(list::get)
            .service(remove::delete)
            .service(run::post)
            .service(push::post)
            .service(deps::get)
            .service(pull::get)
            .service(stop::post)
            .service(print::get)
            .service(cron::get)
            .service(cron::post)
            .service(cron::patch)
            .service(cron::delete)
            .service(resource("/ws-exec/").route(get().to(exec::ws)))
            .service(resource("/ws-monit/").route(get().to(monit::ws)))
            .service(resource("/ws-login/").route(get().to(login::ws)))
    });

    let address = format!("{host}:{port}");
    server = match &config.local.server.tls {
        Some(tls) => {
            let cert_chain = load_server_certificate(&tls.cert_chain)?;
            let private_key = load_server_private_key(&tls.private_key)?;
            let builder = ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(cert_chain, private_key)?;
            server.bind_rustls(address, builder)?
        }
        None => server.bind(address)?,
    };

    server.run().await?;
    Ok(())
}
