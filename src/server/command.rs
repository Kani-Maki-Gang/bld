use crate::config::{definitions::VERSION, BldConfig};
use crate::helpers::term::print_info;
use crate::server::{
    auth_redirect, hist, home, inspect, list, push, stop, ws_exec, ws_monit, PipelinePool,
};
use crate::types::{BldCommand, Result};
use actix::{Arbiter, System};
use actix_web::{middleware, web, App, HttpServer};
use clap::{App as ClapApp, Arg, ArgMatches, SubCommand};

static SERVER: &str = "server";
static HOST: &str = "host";
static PORT: &str = "port";

pub struct ServerCommand;

impl ServerCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }

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
                .service(inspect)
                .service(web::resource("/ws-exec/").route(web::get().to(ws_exec)))
                .service(web::resource("/ws-monit/").route(web::get().to(ws_monit)))
        })
        .bind(format!("{}:{}", host, port))?
        .run()
        .await?;
        Ok(())
    }

    pub fn spawn(config: BldConfig, host: String, port: i64) -> Result<()> {
        let system = System::new("bld-server");
        Arbiter::spawn(async move {
            let _ = Self::start(config, &host, port).await;
        });
        system.run()?;
        Ok(())
    }
}

impl BldCommand for ServerCommand {
    fn id(&self) -> &'static str {
        SERVER
    }

    fn interface(&self) -> ClapApp<'static, 'static> {
        let host = Arg::with_name(HOST)
            .long("host")
            .short("H")
            .help("The server's host address")
            .takes_value(true);
        let port = Arg::with_name(PORT)
            .long("port")
            .short("P")
            .help("The server's port")
            .takes_value(true);
        SubCommand::with_name(SERVER)
            .about("Start bld in server mode, listening to incoming build requests")
            .version(VERSION)
            .args(&[host, port])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> Result<()> {
        let config = BldConfig::load()?;
        let host = matches
            .value_of("host")
            .or(Some(&config.local.host))
            .unwrap()
            .to_string();
        let port = matches
            .value_of("port")
            .map(|port| port.parse::<i64>().unwrap_or(config.local.port))
            .unwrap_or(config.local.port);
        Self::spawn(config, host, port)?;
        Ok(())
    }
}
