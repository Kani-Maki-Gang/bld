use crate::{auth::oauth2::oauth2_login, command::BldCommand};
use anyhow::Result;
use bld_config::{Auth, BldConfig};
use bld_utils::sync::IntoData;
use clap::Args;
use tracing::{debug, level_filters::LevelFilter};

#[derive(Args)]
#[command(about = "Initiates the login process for a bld server")]
pub struct AuthCommand {
    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to login into"
    )]
    server: Option<String>,
}

impl BldCommand for AuthCommand {
    fn verbose(&self) -> bool {
        false
    }

    fn tracing_level(&self) -> LevelFilter {
        LevelFilter::OFF
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_data();
        let config_clone = config.clone();

        let server = config_clone.server_or_first(self.server.as_ref())?;
        let server_auth = config_clone.same_auth_as(server)?;
        let server_name = server.name.to_owned().into_data();

        debug!("running login subcommand with --server: {}", server.name);

        match &server_auth.auth {
            Some(Auth::OAuth2(_)) => {
                debug!(
                    "starting login process for server: {} with oauth2 method",
                    server.name
                );
                oauth2_login(config, server_name)
            }
            _ => unimplemented!(),
        }
    }
}
