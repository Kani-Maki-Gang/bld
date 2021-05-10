use crate::config::{definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_get, headers};
use crate::types::{BldCommand, Result};
use clap::{App, Arg, ArgMatches, SubCommand};

static LIST: &str = "ls";
static SERVER: &str = "server";

pub struct ListCommand;

impl ListCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for ListCommand {
    fn id(&self) -> &'static str {
        LIST
    }

    fn interface(&self) -> App<'static, 'static> {
        let server = Arg::with_name(SERVER)
            .short("s")
            .long("server")
            .takes_value(true)
            .help("The name of the server from which to fetch pipeline information");
        SubCommand::with_name(LIST)
            .about("Lists information of pipelines in a bld server")
            .version(VERSION)
            .args(&vec![server])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> Result<()> {
        let config = BldConfig::load()?;
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let sys = String::from("bld-ls");
        let url = format!("http://{}:{}/list", srv.host, srv.port);
        let headers = headers(name, auth)?;
        exec_get(sys, url, headers);
        Ok(())
    }
}

