use crate::config::{Auth, AuthValidation, BldConfig, BldLocalConfig, BldRemoteConfig};
use crate::helpers::term;
use crate::types::Result;
use clap::ArgMatches;

fn list_locals(local: &BldLocalConfig) -> Result<()> {
    term::print_info("Local configuration:")?;
    println!("- server-mode: {}", local.server_mode);
    match &local.auth {
        AuthValidation::OAuth2(url) => {
            println!("- auth:");
            println!("  - method: oauth2");
            println!("  - validation-url: {}", url);
        }
        AuthValidation::Ldap => {
            println!("- auth:");
            println!("  - method: ldap");
        }
        _ => {}
    }
    println!("- host: {}", local.host);
    println!("- port: {}", local.port);
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
                println!("  - auth-url: {}", info.auth_url.to_string());
                println!("  - token-url: {}", info.token_url.to_string());
                println!("  - redirect-url: {}", info.redirect_url.to_string());
                println!("  - client-id: {}", info.client_id.to_string());
                println!("  - client-secret: ***********");
                println!(
                    "  - scopes: [{} ]",
                    info.scopes.iter().fold(String::new(), |acc, n| format!(
                        "{} \"{}\",",
                        acc,
                        n.to_string()
                    ))
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
    list_locals(&config.local)?;
    println!();
    list_remote(&config.remote)?;
    Ok(())
}

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    if matches.is_present("local") {
        return list_locals(&config.local);
    }
    if matches.is_present("remote") {
        return list_remote(&config.remote);
    }
    list_all(&config)
}
