use crate::config::{definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_post, headers};
use crate::types::{BldCommand, Result};
use clap::{App, Arg, ArgMatches, SubCommand};

static STOP: &str = "stop";
static ID: &str = "id";
static SERVER: &str = "server";

pub struct StopCommand;

impl StopCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for StopCommand {
    fn id(&self) -> &'static str {
        STOP
    }

    fn interface(&self) -> App<'static, 'static> {
        let id = Arg::with_name(ID)
            .short("i")
            .long("id")
            .help("The id of a pipeline running on a server")
            .required(true)
            .takes_value(true);
        let server = Arg::with_name(SERVER)
            .short("s")
            .long("server")
            .help("The name of the server that the pipeline is running")
            .takes_value(true);
        SubCommand::with_name(STOP)
            .about("Stops a running pipeline on a server")
            .version(VERSION)
            .args(&[id, server])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> Result<()> {
        let config = BldConfig::load()?;
        let id = matches.value_of(ID).unwrap().to_string();
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let sys = String::from("bld-stop");
        let url = format!("http://{}:{}/stop", srv.host, srv.port);
        let headers = headers(name, auth)?;
        exec_post(sys, url, headers, id);
        Ok(())
    }
}
