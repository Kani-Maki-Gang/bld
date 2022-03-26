use crate::cli::BldCommand;
use crate::config::{definitions::TOOL_DEFAULT_PIPELINE, definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request;
use crate::push::PushInfo;
use crate::run::Pipeline;
use actix_web::rt::System;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::collections::HashSet;
use tracing::debug;

static PUSH: &str = "push";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";

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

    fn interface(&self) -> App<'static, 'static> {
        let pipeline = Arg::with_name(PIPELINE)
            .short("p")
            .long("pipeline")
            .help("The name of the pipeline to push")
            .takes_value(true);
        let server = Arg::with_name(SERVER)
            .short("s")
            .long("server")
            .help("The name of the server to push changes to")
            .takes_value(true);
        SubCommand::with_name(PUSH)
            .about("Pushes the contents of a pipeline to a bld server")
            .version(VERSION)
            .args(&[pipeline, server])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        System::new().block_on(async move { do_push(matches).await })
    }
}

async fn do_push(matches: &ArgMatches<'_>) -> anyhow::Result<()> {
    let config = BldConfig::load()?;
    let pip = matches
        .value_of(PIPELINE)
        .unwrap_or(TOOL_DEFAULT_PIPELINE)
        .to_string();
    let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
    debug!("running {} subcommand with --server: {}", PUSH, srv.name);
    let (name, auth) = match &srv.same_auth_as {
        Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
            Some(srv) => (&srv.name, &srv.auth),
            None => return auth_for_server_invalid(),
        },
        None => (&srv.name, &srv.auth),
    };
    let payload = build_payload(pip)?;
    let data: Vec<PushInfo> = payload
        .iter()
        .map(|(n, s)| {
            println!("Pushing {n}...");
            PushInfo::new(n, s)
        })
        .collect();
    let url = format!("http://{}:{}/push", srv.host, srv.port);
    let headers = request::headers(name, auth)?;
    debug!("sending http request to {}", url);
    request::post(url, headers, data).await.map(|r| {
        println!("{r}");
    })
}

fn build_payload(name: String) -> anyhow::Result<HashSet<(String, String)>> {
    let src = Pipeline::read(&name)?;
    let pipeline = Pipeline::parse(&src)?;
    let mut set = HashSet::new();
    set.insert((name, src));
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
