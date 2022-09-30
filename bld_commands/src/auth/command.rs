use crate::auth::Login;
use crate::BldCommand;
use anyhow::anyhow;
use bld_config::{definitions::VERSION, Auth, BldConfig};
use bld_utils::errors::auth_for_server_invalid;
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

    fn exec(&self, matches: &ArgMatches) -> anyhow::Result<()> {
        let config = BldConfig::load()?;
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        debug!("running {} subcommand with --server: {}", LOGIN, &srv.name);
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        match auth {
            Auth::OAuth2(info) => {
                debug!(
                    "starting login process for server: {} with oauth2 method",
                    name
                );
                info.login(name)?;
                Ok(())
            }
            Auth::Ldap => Err(anyhow!("unsupported authentication method ldap")),
            Auth::None => Err(anyhow!("no authentication method setup")),
        }
    }
}
