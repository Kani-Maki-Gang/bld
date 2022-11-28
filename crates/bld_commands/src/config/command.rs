use crate::command::BldCommand;
use anyhow::Result;
use bld_config::{Auth, AuthValidation, BldConfig, BldLocalConfig, BldRemoteServerConfig};
use bld_utils::term;
use clap::Args;

#[derive(Args)]
#[command(about = "Lists bld's configuration")]
pub struct ConfigCommand;

impl ConfigCommand {
    fn list_locals(local: &BldLocalConfig) -> Result<()> {
        term::print_info("Local configuration:")?;
        match &local.auth {
            Some(AuthValidation::OAuth2(url)) => {
                println!("- auth:");
                println!("  - method: oauth2");
                println!("  - validation-url: {url}");
            }
            Some(AuthValidation::Ldap) => {
                println!("- auth:");
                println!("  - method: ldap");
            }
            _ => {}
        }
        println!("- server:");
        println!("  - host: {}", local.server.host);
        println!("  - port: {}", local.server.port);
        println!("  - pipelines: {}", local.server.pipelines);

        match &local.server.tls {
            Some(tls) => {
                println!("  - tls:");
                println!("    - cert-chain:  {}", tls.cert_chain);
                println!("    - private-key: {}", tls.private_key);
            }
            None => println!("  - tls: None"),
        }

        println!("- supervisor:");
        println!("  - host: {}", local.supervisor.host);
        println!("  - port: {}", local.supervisor.port);
        println!("  - workers: {}", local.supervisor.workers);

        match &local.supervisor.tls {
            Some(tls) => {
                println!("  - tls:");
                println!("    - cert-chain:  {}", tls.cert_chain);
                println!("    - private-key: {}", tls.private_key);
            }
            None => println!("  - tls: None"),
        }

        println!("- logs: {}", local.logs);
        println!("- db: {}", local.db);
        println!("- docker-url: {}", local.docker_url);
        Ok(())
    }

    fn list_remote(remote: &[BldRemoteServerConfig]) -> Result<()> {
        term::print_info("Remote configuration:")?;

        for (i, server) in remote.iter().enumerate() {
            println!("- name: {}", server.name);
            println!("- host: {}", server.host);
            println!("- port: {}", server.port);
            println!("- tls:  {}", server.tls);

            match &server.auth {
                Some(Auth::OAuth2(info)) => {
                    println!("- auth:");
                    println!("  - method: oauth2");
                    println!("  - auth-url: {}", *info.auth_url);
                    println!("  - token-url: {}", *info.token_url);
                    println!("  - redirect-url: {}", *info.redirect_url);
                    println!("  - client-id: {}", *info.client_id);
                    println!("  - client-secret: ***********");
                    println!(
                        "  - scopes: [{} ]",
                        info.scopes
                            .iter()
                            .fold(String::new(), |acc, n| format!("{acc} \"{}\",", **n))
                    );
                }
                Some(Auth::Ldap) => {
                    println!("- auth:");
                    println!("  - method: ldap");
                }
                _ => {}
            }

            if i < remote.len() - 1 {
                println!();
            }
        }

        Ok(())
    }

    fn list_all(config: &BldConfig) -> Result<()> {
        Self::list_locals(&config.local)?;
        println!();
        Self::list_remote(&config.remote)?;
        Ok(())
    }
}

impl BldCommand for ConfigCommand {
    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?;
        Self::list_all(&config)
    }
}
