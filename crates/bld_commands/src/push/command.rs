use crate::BldCommand;
use actix_web::rt::System;
use anyhow::{anyhow, Result};
use bld_config::BldConfig;
use bld_config::{definitions::VERSION, BldRemoteServerConfig};
use bld_core::proxies::PipelineFileSystemProxy;
use bld_runner::Pipeline;
use bld_server::requests::PushInfo;
use bld_utils::request::Request;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::collections::HashMap;
use tracing::debug;

static PUSH: &str = "push";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";
static IGNORE_DEPS: &str = "ignore-deps";

pub struct PushCommand;

impl BldCommand for PushCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        PUSH
    }

    fn interface(&self) -> Command {
        let pipeline = Arg::new(PIPELINE)
            .short('p')
            .long("pipeline")
            .help("The name of the pipeline to push")
            .action(ArgAction::Set)
            .required(true);

        let server = Arg::new(SERVER)
            .short('s')
            .long("server")
            .help("The name of the server to push changes to")
            .action(ArgAction::Set);

        let ignore = Arg::new(IGNORE_DEPS)
            .long(IGNORE_DEPS)
            .help("Don't include other pipeline dependencies")
            .action(ArgAction::SetTrue);

        Command::new(PUSH)
            .about("Pushes the contents of a pipeline to a bld server")
            .version(VERSION)
            .args(&[pipeline, server, ignore])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        // using unwrap here because the pipeline option is required.
        let pip = matches.get_one::<String>(PIPELINE).cloned().unwrap();
        let server = config
            .remote
            .server_or_first(matches.get_one::<String>(SERVER))?;
        let ignore = matches.get_flag(IGNORE_DEPS);

        debug!(
            "running {PUSH} subcommand with --server: {} and --pipeline: {pip}",
            server.name
        );

        let server_auth = config.remote.same_auth_as(server)?;

        System::new().block_on(async move {
            do_push(
                server.host.clone(),
                server.port,
                server.http_protocol(),
                server_auth.clone(),
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
    server_auth: BldRemoteServerConfig,
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
        debug!("sending request to {url}");

        let _ = Request::post(&url)
            .auth(&server_auth)
            .send_json(info)
            .await
            .map(|_: String| {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_push_pipeline_arg_accepts_value() {
        let pipeline_name = "mock_pipeline_name";
        let command = PushCommand::boxed().interface();
        let matches = command.get_matches_from(&["push", "-p", pipeline_name]);

        assert_eq!(
            matches.get_one::<String>(PIPELINE),
            Some(&pipeline_name.to_string())
        )
    }

    #[test]
    fn cli_push_server_arg_accepts_value() {
        let server_name = "mock_server_name";
        let command = PushCommand::boxed().interface();
        let matches = command.get_matches_from(&["push", "-p", "mockPipeline", "-s", server_name]);

        assert_eq!(
            matches.get_one::<String>(SERVER),
            Some(&server_name.to_string())
        )
    }

    #[test]
    fn cli_push_ignore_deps_is_a_flag() {
        let command = PushCommand::boxed().interface();
        let matches = command.get_matches_from(&["push", "-p", "mockPipeline", "--ignore-deps"]);

        assert_eq!(matches.get_flag(IGNORE_DEPS), true);
    }
}
