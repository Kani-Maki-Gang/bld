use crate::cron::CronScheduler;
use crate::endpoints::auth::WebCoreClient;
use crate::endpoints::{
    auth, check, copy, cron, deps, hist, home, list, print, pull, push, r#move, remove, run, stop,
    ui,
};
use crate::sockets::{exec, login, monit};
use crate::supervisor::channel::SupervisorMessageSender;
use actix_cors::Cors;
use actix_web::{
    middleware,
    web::{get, resource},
    App, HttpServer,
};
use anyhow::Result;
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_models::new_connection_pool;
use bld_utils::{
    sync::IntoData,
    tls::{load_server_certificate, load_server_private_key},
};
use rustls::ServerConfig;
use std::{env::set_var, sync::Arc};
use tracing::info;

pub async fn start(config: BldConfig, host: String, port: i64) -> Result<()> {
    info!("starting bld server at {}:{}", host, port);

    let config = config.into_data();
    let client = config.openid_core_client().await?.into_data();
    let web_client = WebCoreClient(config.openid_web_core_client().await?).into_data();
    let config_clone = config.clone();
    let conn = new_connection_pool(Arc::clone(&config)).await?;
    let supervisor_sender = SupervisorMessageSender::new(Arc::clone(&config)).into_data();
    let pool = conn.into_data();
    let fs = FileSystem::server(Arc::clone(&config), Arc::clone(&pool)).into_data();
    let cron = CronScheduler::new(
        Arc::clone(&fs),
        Arc::clone(&pool),
        Arc::clone(&supervisor_sender),
    )
    .await?
    .into_data();

    set_var("RUST_LOG", "actix_server=info,actix_web=debug");
    let mut server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .app_data(config_clone.clone())
            .app_data(client.clone())
            .app_data(web_client.clone())
            .app_data(supervisor_sender.clone())
            .app_data(pool.clone())
            .app_data(fs.clone())
            .app_data(cron.clone())
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(auth::available)
            .service(auth::redirect)
            .service(auth::refresh)
            .service(auth::web_client_start)
            .service(auth::web_client_validate)
            .service(check::get)
            .service(copy::post)
            .service(hist::get)
            .service(list::get)
            .service(remove::delete)
            .service(run::post)
            .service(push::post)
            .service(deps::get)
            .service(pull::get)
            .service(stop::post)
            .service(r#move::patch)
            .service(print::get)
            .service(cron::get)
            .service(cron::post)
            .service(cron::patch)
            .service(cron::delete)
            .service(ui::queued_pipelines)
            .service(ui::running_pipelines)
            .service(resource("/v1/ws-exec/").route(get().to(exec::ws)))
            .service(resource("/v1/ws-monit/").route(get().to(monit::ws)))
            .service(resource("/v1/ws-login/").route(get().to(login::ws)))
            .service(home::index)
            .service(home::fallback)
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
