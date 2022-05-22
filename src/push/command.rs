use crate::cli::BldCommand;
use crate::config::{definitions::TOOL_DEFAULT_PIPELINE, definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request;
use crate::push::PushInfo;
use crate::run::Pipeline;
use actix_web::rt::System;
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
        let ignore = Arg::with_name(IGNORE_DEPS)
            .long(IGNORE_DEPS)
            .help("Don't include other pipeline dependencies")
            .takes_value(false);
        SubCommand::with_name(PUSH)
            .about("Pushes the contents of a pipeline to a bld server")
            .version(VERSION)
            .args(&[pipeline, server, ignore])
    }

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()> {
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
            do_push(srv.host.clone(), srv.port, headers, pip, ignore).await 
        })
    }
}

async fn do_push(host: String, port: i64, headers: HashMap<String, String>, name: String, ignore_deps: bool) -> anyhow::Result<()> {
    let mut pipelines = vec![PushInfo::new(&name, &Pipeline::read(&name)?)];
    if !ignore_deps {
        print!("Resolving dependecies...");
        let mut deps = Pipeline::deps(&name)
            .map(|pips| {
                println!("Done.");
                pips.iter()
                    .map(|(n, s)| PushInfo::new(n, s))
                    .collect()
            })
            .map_err(|e| {
                println!("Error. {e}");
                e
            })?;
        pipelines.append(&mut deps);
    }
    for info in pipelines.into_iter() {
        print!("Pushing {}...", info.name);
        let url = format!("http://{}:{}/push", host, port);
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
