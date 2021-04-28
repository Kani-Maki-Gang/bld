use crate::config::BldConfig;
use crate::helpers::term::print_info;
use crate::server::{auth_redirect, hist, home, list, push, stop, ws_exec, ws_monit, PipelinePool};
use crate::types::Result;
use actix::{Arbiter, System};
use actix_web::{middleware, web, App, HttpServer};
use clap::ArgMatches;

async fn start(config: BldConfig, host: &str, port: i64) -> Result<()> {
    print_info(&format!("starting bld server at {}:{}", host, port))?;
    let config_data = web::Data::new(config);
    let pool_data = web::Data::new(PipelinePool::new());
    std::env::set_var("RUST_LOG", "actix_server=info,actix_wev=info");
    env_logger::init();
    HttpServer::new(move || {
        App::new()
            .app_data(pool_data.clone())
            .app_data(config_data.clone())
            .wrap(middleware::Logger::default())
            .service(home)
            .service(auth_redirect)
            .service(hist)
            .service(list)
            .service(push)
            .service(stop)
            .service(web::resource("/ws-exec/").route(web::get().to(ws_exec)))
            .service(web::resource("/ws-monit").route(web::get().to(ws_monit)))
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await?;
    Ok(())
}

pub fn sys_spawn(config: BldConfig, host: String, port: i64) -> Result<()> {
    let system = System::new("bld-server");
    Arbiter::spawn(async move {
        let _ = start(config, &host, port).await;
    });
    system.run()?;
    Ok(())
}

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let host = matches
        .value_of("host")
        .or(Some(&config.local.host))
        .unwrap()
        .to_string();
    let port = match matches.value_of("port") {
        Some(port) => match port.parse::<i64>() {
            Ok(port) => port,
            Err(_) => config.local.port,
        },
        None => config.local.port,
    };
    sys_spawn(config, host, port)?;
    Ok(())
}
