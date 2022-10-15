use crate::auth::Login;
use crate::BldCommand;
use anyhow::{anyhow, Result};
use bld_config::{definitions::VERSION, Auth, BldConfig};
use clap::{App, Arg, ArgMatches, SubCommand};
use tracing::debug;

static LOGIN: &str = "login";
static SERVER: &str = "server";

pub struct AuthCommand;

impl AuthCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for AuthCommand {
    fn id(&self) -> &'static str {
        LOGIN
    }

    fn interface(&self) -> App<'static> {
        let server = Arg::with_name(SERVER)
            .short('s')
            .long("server")
            .help("The target bld server")
            .takes_value(true);
        SubCommand::with_name(LOGIN)
            .about("Initiates the login process for a bld server")
            .version(VERSION)
            .args(&[server])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config.remote.server_or_first(matches.value_of(SERVER))?;
        let server_auth = config.remote.same_auth_as(server)?;
        debug!(
            "running {} subcommand with --server: {}",
            LOGIN, &server.name
        );
        match &server_auth.auth {
            Auth::OAuth2(info) => {
                debug!(
                    "starting login process for server: {} with oauth2 method",
                    server.name
                );
                info.login(&server_auth.name)?;
                Ok(())
            }
            Auth::Ldap => Err(anyhow!("unsupported authentication method ldap")),
            Auth::None => Err(anyhow!("no authentication method setup")),
        }
    }
}
