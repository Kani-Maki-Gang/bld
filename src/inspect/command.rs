use crate::config::{definitions::TOOL_DEFAULT_PIPELINE, definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_get, headers};
use crate::types::BldCommand;
use clap::{App, Arg, ArgMatches, SubCommand};

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
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let url = format!("http://{}:{}/inspect/{}", srv.host, srv.port, pip);
        let headers = headers(name, auth)?;
        let sys = String::from("bld-inspect");
        exec_get(sys, url, headers);
        Ok(())
    }
}
