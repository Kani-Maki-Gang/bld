use crate::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::{definitions::TOOL_DEFAULT_PIPELINE, definitions::VERSION, BldConfig};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::Pipeline;
use bld_server::requests::PushInfo;
use bld_utils::errors::auth_for_server_invalid;
use bld_utils::request;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::collections::HashMap;
use tracing::debug;

static PUSH: &str = "push";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";
static IGNORE_DEPS: &str = "ignore-deps";

pub struct PushCommand;

impl PushCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for PushCommand {
    fn id(&self) -> &'static str {
        PUSH
    }

    fn interface(&self) -> App<'static> {
        let pipeline = Arg::with_name(PIPELINE)
            .short('p')
            .long("pipeline")
            .help("The name of the pipeline to push")
            .takes_value(true);
        let server = Arg::with_name(SERVER)
            .short('s')
            .long("server")
            .help("The name of the server to push changes to")
            .takes_value(true);
        let ignore = Arg::with_name(IGNORE_DEPS)
            .long(IGNORE_DEPS)
            .help("Don't include other pipeline dependencies")
            .takes_value(false);
        SubCommand::with_name(PUSH)
            .about("Pushes the contents of a pipeline to a bld server")
            .version(VERSION)
            .args(&[pipeline, server, ignore])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let pip = matches
            .value_of(PIPELINE)
            .unwrap_or(TOOL_DEFAULT_PIPELINE)
            .to_string();
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        let ignore = matches.is_present(IGNORE_DEPS);
        debug!(
            "running {PUSH} subcommand with --server: {} and --pipeline: {pip}",
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
            do_push(
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

async fn do_push(
    host: String,
    port: i64,
    protocol: String,
    headers: HashMap<String, String>,
    name: String,
    ignore_deps: bool,
) -> Result<()> {
    let mut pipelines = vec![PushInfo::new(
        &name,
        &PipelineFileSystemProxy::Local.read(&name)?,
    )];
    if !ignore_deps {
        print!("Resolving dependecies...");
        let mut deps = deps(&name)
            .map(|pips| {
                println!("Done.");
                pips.iter().map(|(n, s)| PushInfo::new(n, s)).collect()
            })
            .map_err(|e| {
                println!("Error. {e}");
                e
            })?;
        pipelines.append(&mut deps);
    }
    for info in pipelines.into_iter() {
        print!("Pushing {}...", info.name);
        let url = format!("{protocol}://{}:{}/push", host, port);
        debug!("sending http request to {}", url);
        let _ = request::post(url.clone(), headers.clone(), info)
            .await
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

fn deps(name: &str) -> Result<HashMap<String, String>> {
    deps_recursive(name).map(|mut hs| {
        hs.remove(name);
        hs.into_iter().collect()
    })
}

fn deps_recursive(name: &str) -> Result<HashMap<String, String>> {
    debug!("Parsing pipeline {name}");
    let src = PipelineFileSystemProxy::Local
        .read(name)
        .map_err(|_| anyhow!("Pipeline {name} not found"))?;
    let pipeline = Pipeline::parse(&src)?;
    let mut set = HashMap::new();
    set.insert(name.to_string(), src);
    for step in pipeline.steps.iter() {
        for call in &step.call {
            let subset = deps_recursive(call)?;
            for (k, v) in subset {
                set.insert(k, v);
            }
        }
    }
    Ok(set)
}
