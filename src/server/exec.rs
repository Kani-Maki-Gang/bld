use actix_web::{HttpServer, App, Responder, HttpResponse, get};
use crate::{config::BldConfig, term::print_info};
use clap::ArgMatches;
use std::io;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Bld server running")
}

async fn start(host: &str, port: i64) -> io::Result<()> {
    print_info(&format!("starting bld server at {}:{}", host, port))?;

    HttpServer::new(|| {
        App::new().service(hello)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

pub async fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let config = BldConfig::load()?;

    let host = match matches.value_of("host") {
        Some(host) => host,
        None => &config.local.host,
    };

    let port = match matches.value_of("port") {
        Some(port) => match port.parse::<i64>() {
            Ok(port) => port,
            Err(_) => config.local.port,
        },
        None => config.local.port,
    };

    start(host, port).await
}