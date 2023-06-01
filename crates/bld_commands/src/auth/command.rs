use std::{fmt::Write, process::ExitStatus};

use crate::command::BldCommand;
use actix::{spawn, System};
use actix_web::web::Data;
use anyhow::{anyhow, bail, Result};
use bld_config::{Auth, BldConfig};
use bld_utils::sync::IntoData;
use clap::Args;
use oauth2::{basic::BasicClient, CsrfToken, PkceCodeChallenge};
use tokio::{process::Command, sync::mpsc::channel};
use tracing::{debug, level_filters::LevelFilter};

use super::oauth2::oauth2_server;

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

impl AuthCommand {
    async fn oauth2_prompt(config: Data<BldConfig>, server: Data<String>) -> Result<()> {
        let Some(Auth::OAuth2(oauth2)) = &config.server(&server)?.auth else {
            bail!("server isn't configured for oauth2 authentication");
        };

        let client = BasicClient::new(
            oauth2.client_id.clone(),
            Some(oauth2.client_secret.clone()),
            oauth2.auth_url.clone(),
            Some(oauth2.token_url.clone()),
        )
        .set_redirect_uri(oauth2.redirect_url.clone());

        let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_url = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        for scope in &oauth2.scopes {
            auth_url = auth_url.add_scope(scope.clone());
        }

        let (auth_url, _) = auth_url.url();

        let mut cmd = Command::new("xdg-open");
        cmd.arg(auth_url.to_string());

        let status = cmd.status().await?;

        let mut message = String::new();
        if !ExitStatus::success(&status) {
            writeln!(message, "Use the below url in order to login:")?;
            writeln!(message)?;
            writeln!(message)?;
            writeln!(message, "{auth_url}")?;
        } else {
            writeln!(
                message,
                "A new browser tab has opened in order to complete the login process."
            )?;
        }

        println!("{message}");

        Ok(())
    }

    fn oauth2_login(config: Data<BldConfig>, server: Data<String>) -> Result<()> {
        System::new().block_on(async move {
            let (tx, mut rx) = channel(4096);
            let tx = tx.into_data();
            let config_clone = config.clone();
            let server_clone = server.clone();

            spawn(async move { oauth2_server(tx, config_clone, server_clone).await });

            Self::oauth2_prompt(config, server).await?;

            rx.recv()
                .await
                .ok_or_else(|| anyhow!("Unable to retrieve login result."))
                .map(|x| {
                    if x.is_ok() {
                        println!("Login completed successfully!");
                    }
                    x
                })?
        })
    }
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
                Self::oauth2_login(config, server_name)
            }
            Some(Auth::Ldap) => bail!("unsupported authentication method ldap"),
            _ => bail!("no authentication method setup"),
        }
    }
}
