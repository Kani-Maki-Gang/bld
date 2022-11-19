use crate::auth::Login;
use crate::command::BldCommand;
use anyhow::{bail, Result};
use bld_config::{Auth, BldConfig};
use clap::Args;
use tracing::debug;

#[derive(Args)]
#[command(about = "Initiates the login process for a bld server")]
pub struct AuthCommand {
    #[arg(short = 's', long = "server", help = "The target bld server")]
    server: Option<String>,
}

impl BldCommand for AuthCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(self.server.as_ref())?;
        let server_auth = config.remote.same_auth_as(server)?;

        debug!(
            "running login subcommand with --server: {}",
            server.name
        );

        match &server_auth.auth {
            Auth::OAuth2(info) => {
                debug!(
                    "starting login process for server: {} with oauth2 method",
                    server.name
                );
                info.login(&server_auth.name).map(|_| ())
            }
            Auth::Ldap => bail!("unsupported authentication method ldap"),
            Auth::None => bail!("no authentication method setup"),
        }
    }
}
