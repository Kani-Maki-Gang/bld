use crate::cli::BldCommand;
use crate::config::{definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::fs::IsYaml;
use crate::helpers::request;
use crate::pull::{PullRequestInfo, PullResponseInfo};
use crate::run::Pipeline;
use actix_web::rt::System;
use anyhow::anyhow;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::fs::{create_dir_all, remove_file, File};
use std::io::Write;
use tracing::debug;

const PULL: &str = "pull";
const SERVER: &str = "server";
const PIPELINE: &str = "pipeline";
const IGNORE_DEPS: &str = "ignore-deps";

pub struct PullCommand;

impl PullCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for PullCommand {
    fn id(&self) -> &'static str {
        PULL
    }

    fn interface(&self) -> App<'static, 'static> {
        let server = Arg::with_name(SERVER)
            .short("s")
            .long(SERVER)
            .help("The name of the bld server")
            .takes_value(true);
        let pipeline = Arg::with_name(PIPELINE)
            .short("p")
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

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        System::new().block_on(async move { do_pull(matches).await })
    }
}

async fn do_pull(matches: &ArgMatches<'_>) -> anyhow::Result<()> {
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
    let url = format!("http://{}:{}/pull", srv.host, srv.port);
    let headers = request::headers(name, auth)?;
    let body = PullRequestInfo::new(&pip, !ignore);
    debug!("sending http request to {}", url);
    request::post(url, headers, body)
        .await
        .and_then(|r| serde_json::from_str::<Vec<PullResponseInfo>>(&r).map_err(|e| anyhow!(e)))
        .and_then(save_pipelines)
}

fn save_pipelines(pipelines: Vec<PullResponseInfo>) -> anyhow::Result<()> {
    for entry in pipelines.iter() {
        let path = Pipeline::get_path(&entry.name)?;
        if path.is_yaml() {
            remove_file(&path)?;
        } else if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let mut handle = File::create(&path)?;
        handle.write_all(entry.content.as_bytes())?;
    }
    Ok(())
}
