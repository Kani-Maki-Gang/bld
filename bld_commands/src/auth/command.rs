use crate::auth::Login;
use crate::BldCommand;
use anyhow::{anyhow, Result};
use bld_config::{definitions::VERSION, Auth, BldConfig};
use clap::{Arg, ArgAction, ArgMatches, Command};
use tracing::debug;

static LOGIN: &str = "login";
static SERVER: &str = "server";

pub struct AuthCommand;

impl BldCommand for AuthCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        LOGIN
    }

    fn interface(&self) -> Command {
        let server = Arg::new(SERVER)
            .short('s')
            .long("server")
            .help("The target bld server")
            .action(ArgAction::Append);

        Command::new(LOGIN)
            .about("Initiates the login process for a bld server")
            .version(VERSION)
            .args(&[server])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_auth_server_arg_accepts_value() {
        let server_name = "mock_server_name";
        let command = AuthCommand::boxed().interface();
        let matches = command.get_matches_from(&["login", "-s", server_name]);

        assert_eq!(
            matches.get_one::<String>(SERVER),
            Some(&server_name.to_string())
        );
    }
}
