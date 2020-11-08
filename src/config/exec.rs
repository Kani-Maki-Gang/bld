use crate::config::{BldConfig, BldLocalConfig, BldRemoteConfig};
use crate::term;
use clap::ArgMatches;
use std::io;

fn list_locals(local: &BldLocalConfig) -> io::Result<()> {
    term::print_info("Local configuration:")?;
    println!("- enable_server: {}", local.enable_server);
    println!("- host: {}", local.host);
    println!("- port: {}", local.port);
    println!("- logs: {}", local.logs);
    println!("- docker_host: {}", local.docker_host);
    println!("- docker_port: {}", local.docker_port);
    println!("- docker_use_tls: {}", local.docker_use_tls);
    Ok(())
}

fn list_remote(remote: &BldRemoteConfig) -> io::Result<()> {
    term::print_info("Remote configuration:")?;

    for (i, server) in remote.servers.iter().enumerate() {
        println!("- name: {}", server.name);
        println!("- host: {}", server.host);
        println!("- port: {}", server.port);
        if i < remote.servers.len() - 1 {
            println!("");
        }
    }

    Ok(())
}

fn list_all(config: &BldConfig) -> io::Result<()> {
    list_locals(&config.local)?;
    println!("");
    list_remote(&config.remote)?;
    Ok(())
}

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let config = BldConfig::load()?;

    if matches.is_present("local") {
        return list_locals(&config.local);
    }

    if matches.is_present("remote") {
        return list_remote(&config.remote);
    }

    list_all(&config)
}
