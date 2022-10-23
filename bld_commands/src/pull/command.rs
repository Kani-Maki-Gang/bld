use crate::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::{definitions::VERSION, BldConfig};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_server::responses::PullResponse;
use bld_utils::fs::IsYaml;
use bld_utils::request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::collections::HashMap;
use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use tracing::debug;

const PULL: &str = "pull";
const SERVER: &str = "server";
const PIPELINE: &str = "pipeline";
const IGNORE_DEPS: &str = "ignore-deps";

pub struct PullCommand;

impl BldCommand for PullCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        PULL
    }

    fn interface(&self) -> Command {
        let server = Arg::new(SERVER)
            .short('s')
            .long(SERVER)
            .help("The name of the bld server")
            .action(ArgAction::Set);

        let pipeline = Arg::new(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("The name of the pipeline")
            .required(true)
            .action(ArgAction::Set);

        let ignore_deps = Arg::new(IGNORE_DEPS)
            .long(IGNORE_DEPS)
            .help("Do not include other pipeline dependencies")
            .action(ArgAction::SetTrue);

        Command::new(PULL)
            .about("Pull a pipeline from a bld server and stores it localy")
            .version(VERSION)
            .args(&[server, pipeline, ignore_deps])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;
        // using an unwrap here because the pipeline option is required.
        let pip = matches.get_one::<String>(PIPELINE).cloned().unwrap();
        let ignore = matches.get_flag(IGNORE_DEPS);

        debug!(
            "running {PULL} subcommand with --server: {}, --pipeline: {pip} and --ignore-deps: {ignore}",
            server.name
        );

        let server_auth = config.remote.same_auth_as(server)?;
        let headers = request::headers(&server_auth.name, &server_auth.auth)?;

        System::new().block_on(async move {
            do_pull(
                server.host.clone(),
                server.port,
                server.http_protocol(),
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
