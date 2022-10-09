use crate::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::{definitions::VERSION, BldConfig};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_server::responses::PullResponse;
use bld_utils::errors::auth_for_server_invalid;
use bld_utils::fs::IsYaml;
use bld_utils::request;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::collections::HashMap;
use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use tracing::debug;

const PULL: &str = "pull";
const SERVER: &str = "server";
const PIPELINE: &str = "pipeline";
const IGNORE_DEPS: &str = "ignore-deps";

pub struct PullCommand;

impl PullCommand {
    pub fn boxed() -> Box<Self> {
        Box::new(Self)
    }
}

impl BldCommand for PullCommand {
    fn id(&self) -> &'static str {
        PULL
    }

    fn interface(&self) -> App<'static> {
        let server = Arg::with_name(SERVER)
            .short('s')
            .long(SERVER)
            .help("The name of the bld server")
            .takes_value(true);
        let pipeline = Arg::with_name(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("The name of the pipeline")
            .takes_value(true);
        let ignore_deps = Arg::with_name(IGNORE_DEPS)
            .long(IGNORE_DEPS)
            .help("Do not include other pipeline dependencies")
            .takes_value(false);
        SubCommand::with_name(PULL)
            .about("Pull a pipeline from a bld server and stores it localy")
            .version(VERSION)
            .args(&[server, pipeline, ignore_deps])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        let pip = matches
            .value_of(PIPELINE)
            .ok_or_else(|| anyhow!("no pipeline provided"))?
            .to_string();
        let ignore = matches.is_present(IGNORE_DEPS);
        debug!(
            "running {PULL} subcommand with --server: {}, --pipeline: {pip} and --ignore-deps: {ignore}",
            srv.name
        );
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        let headers = request::headers(name, auth)?;
        System::new().block_on(async move {
            do_pull(
                srv.host.clone(),
                srv.port,
                srv.http_protocol(),
                headers,
                pip,
                ignore,
            )
            .await
        })
    }
}

async fn do_pull(
    host: String,
    port: i64,
    protocol: String,
    headers: HashMap<String, String>,
    name: String,
    ignore_deps: bool,
) -> Result<()> {
    let mut pipelines = vec![name.to_string()];
    if !ignore_deps {
        let metadata_url = format!("{protocol}://{host}:{port}/deps");
        debug!("sending http request to {metadata_url}");
        print!("Fetching metadata for dependecies...");
        let mut deps = request::post(metadata_url, headers.clone(), name)
            .await
            .and_then(|r| serde_json::from_str::<Vec<String>>(&r).map_err(|e| anyhow!(e)))
            .map(|d| {
                println!("Done.");
                d
            })
            .map_err(|e| {
                println!("Error. {e}");
                anyhow!(String::new())
            })?;
        pipelines.append(&mut deps);
    }
    for pipeline in pipelines.iter() {
        let url = format!("{protocol}://{host}:{port}/pull");
        debug!("sending http request to {url}");
        print!("Pulling pipeline {pipeline}...");
        let _ = request::post(url, headers.clone(), pipeline.to_string())
            .await
            .and_then(|r| serde_json::from_str(&r).map_err(|e| anyhow!(e)))
            .and_then(save_pipeline)
            .map(|_| {
                println!("Done.");
            })
            .map_err(|e| {
                println!("Error. {e}");
                e
            });
    }
    Ok(())
}

fn save_pipeline(data: PullResponse) -> Result<()> {
    let path = PipelineFileSystemProxy::Local.path(&data.name)?;
    if path.is_yaml() {
        remove_file(&path)?;
    } else if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let mut handle = File::create(&path)?;
    handle.write_all(data.content.as_bytes())?;
    Ok(())
}
