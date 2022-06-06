use crate::BldCommand;
use actix_web::rt::System;
use bld_config::{definitions::TOOL_DEFAULT_PIPELINE, definitions::VERSION, BldConfig};
use bld_utils::errors::auth_for_server_invalid;
use bld_utils::request;
use clap::{App, Arg, ArgMatches, SubCommand};
use tracing::debug;

static INSPECT: &str = "inspect";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";

pub struct InspectCommand;

impl InspectCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for InspectCommand {
    fn id(&self) -> &'static str {
        INSPECT
    }

    fn interface(&self) -> App<'static, 'static> {
        let pipeline = Arg::with_name(PIPELINE)
            .long("pipeline")
            .short("p")
            .help("The name of the pipeline to inspect")
            .takes_value(true);
        let server = Arg::with_name(SERVER)
            .long("server")
            .short("s")
            .help("The name of the server from which to inspect the pipeline")
            .takes_value(true);
        SubCommand::with_name(INSPECT)
            .about("Inspects the contents of a pipeline on a bld server")
            .version(VERSION)
            .args(&[pipeline, server])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let config = BldConfig::load()?;
        let pip = matches
            .value_of(PIPELINE)
            .unwrap_or(TOOL_DEFAULT_PIPELINE)
            .to_string();
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        debug!(
            "running {} subcommand with --pipeline: {}, --server: {}",
            INSPECT, pip, srv.name
        );
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let url = format!("http://{}:{}/inspect", srv.host, srv.port);
        let headers = request::headers(name, auth)?;
        debug!("sending http request to {}", url);
        System::new().block_on(async move {
            request::post(url, headers, pip).await.map(|r| {
                println!("{r}");
            })
        })
    }
}
