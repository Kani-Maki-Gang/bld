use crate::config::{definitions::TOOL_DEFAULT_PIPELINE, BldConfig};
use crate::helpers::errors::{no_server_in_config, server_not_in_config};
use crate::helpers::request::exec_post;
use crate::helpers::term::print_error;
use crate::persist::NullLogger;
use crate::run::Pipeline;
use crate::types::{PushInfo, Result};
use clap::ArgMatches;
use std::collections::HashSet;

fn build_payload(name: String) -> Result<HashSet<(String, String)>> {
    let src = Pipeline::read(&name)?;
    let pipeline = Pipeline::parse(&src, NullLogger::atom())?;
    let mut set = HashSet::new();
    set.insert((name.to_string(), src));
    for step in pipeline.steps.iter() {
        if let Some(pipeline) = &step.call {
            let subset = build_payload(pipeline.to_string())?;
            for entry in subset {
                set.insert(entry);
            }
        }
    }
    Ok(set)
}

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;
    let name = matches
        .value_of("pipeline")
        .or(Some(TOOL_DEFAULT_PIPELINE))
        .unwrap()
        .to_string();
    let (host, port) = match matches.value_of("server") {
        Some(name) => match servers.iter().find(|s| s.name == name) {
            Some(srv) => (&srv.host, srv.port),
            None => return server_not_in_config(),
        },
        None => match servers.iter().next() {
            Some(srv) => (&srv.host, srv.port),
            None => return no_server_in_config(),
        },
    };
    match build_payload(name) {
        Ok(payload) => {
            let sys = String::from("bld-push");
            let data: Vec<PushInfo> = payload
                .iter()
                .map(|(n, s)| {
                    println!("Pushing {}...", n);
                    PushInfo::new(n, s)
                })
                .collect();
            let url = format!("http://{}:{}/push", host, port);
            exec_post(sys, url, data);
        }
        Err(e) => {
            let _ = print_error(&e.to_string());
        }
    }
    Ok(())
}
