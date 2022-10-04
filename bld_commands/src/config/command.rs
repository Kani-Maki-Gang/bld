use crate::BldCommand;
use anyhow::Result;
use bld_config::definitions::VERSION;
use bld_config::{Auth, AuthValidation, BldConfig, BldLocalConfig, BldRemoteConfig};
use bld_utils::term;
use clap::{App, Arg, ArgMatches, SubCommand};

static CONFIG: &str = "config";
static LOCAL: &str = "local";
static REMOTE: &str = "remote";

pub struct ConfigCommand;

impl ConfigCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(ConfigCommand)
    }

    fn list_locals(local: &BldLocalConfig) -> Result<()> {
        term::print_info("Local configuration:")?;
        match &local.auth {
            AuthValidation::OAuth2(url) => {
                println!("- auth:");
                println!("  - method: oauth2");
                println!("  - validation-url: {url}");
            }
            AuthValidation::Ldap => {
                println!("- auth:");
                println!("  - method: ldap");
            }
            _ => {}
        }
        println!("- ha-mode: {}", local.ha_mode);
        println!("- node-id: {:?}", local.node_id);
        println!("- server:");
        println!("  - host: {}", local.server.host);
        println!("  - port: {}", local.server.port);
        println!("  - pipelines: {}", local.server.pipelines);
        println!("- supervisor:");
        println!("  - host: {}", local.supervisor.host);
        println!("  - port: {}", local.supervisor.port);
        println!("  - workers: {}", local.supervisor.workers);
        println!("- logs: {}", local.logs);
        println!("- db: {}", local.db);
        println!("- docker-url: {}", local.docker_url);
        Ok(())
    }

    fn list_remote(remote: &BldRemoteConfig) -> Result<()> {
        term::print_info("Remote configuration:")?;

        for (i, server) in remote.servers.iter().enumerate() {
            println!("- name: {}", server.name);
            println!("- host: {}", server.host);
            println!("- port: {}", server.port);
            match &server.auth {
                Auth::OAuth2(info) => {
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
                Auth::Ldap => {
                    println!("- auth:");
                    println!("  - method: ldap");
                }
                _ => {}
            }
            if i < remote.servers.len() - 1 {
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
    fn id(&self) -> &'static str {
        CONFIG
    }

    fn interface(&self) -> App<'static> {
        let local = Arg::with_name(LOCAL)
            .short('l')
            .long("local")
            .help("List configuration for local options");
        let remote = Arg::with_name(REMOTE)
            .short('r')
            .long("remote")
            .help("List configuration for remote options");
        SubCommand::with_name(CONFIG)
            .about("Lists bld's configuration")
            .version(VERSION)
            .args(&[local, remote])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        if matches.is_present(LOCAL) {
            return Self::list_locals(&config.local);
        }
        if matches.is_present(REMOTE) {
            return Self::list_remote(&config.remote);
        }
        Self::list_all(&config)
    }
}
