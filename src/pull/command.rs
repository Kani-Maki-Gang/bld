use crate::config::{definitions::VERSION, BldConfig};
use crate::cli::BldCommand;
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request;
use anyhow::anyhow;
use actix_web::rt::System;
use clap::{App, Arg, ArgMatches, SubCommand};
use tracing::debug;

const PULL: &str = "pull";
const SERVER: &str = "server";
const PIPELINE: &str = "pipeline";

pub struct PullCommand;

impl PullCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for PullCommand {
   fn id(&self) -> &'static str {
       PULL
   }

   fn interface(&self) -> App<'static, 'static> {
        let server = Arg::with_name(SERVER)
            .short("s")
            .long(SERVER)
            .help("The name of the bld server")
            .takes_value(true);
        let pipeline = Arg::with_name(PIPELINE)
            .short("p")
            .long(PIPELINE)
            .help("the name of the pipeline")
            .takes_value(true);
       SubCommand::with_name(PULL) 
           .about("Pull a pipeline from a bld server and stores it localy")
           .version(VERSION)
           .args(&[server, pipeline])
   }

   fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
       System::new().block_on(async move { do_pull(matches).await })
   }
}

async fn do_pull(matches: &ArgMatches<'_>) -> anyhow::Result<()> {
    let config = BldConfig::load()?;
    let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
    let pip = matches.value_of(PIPELINE).ok_or_else(|| anyhow!("no pipeline provided"))?.to_string();
    debug!(
        "running {PULL} subcommand with --server: {} and --pipeline: {pip}",
        srv.name
    );
    let (name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        },
        None => (&srv.name, &srv.auth),
    };
    let url = format!("http://{}:{}/pull", srv.host, srv.port);
    let headers = request::headers(name, auth)?;
    debug!("sending http request to {}", url);
    request::post(url, headers, pip).await.map(|r| {
        println!("{r}")
    })
}
