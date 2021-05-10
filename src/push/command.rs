use crate::config::{definitions::TOOL_DEFAULT_PIPELINE, definitions::VERSION, BldConfig};
use crate::helpers::errors::auth_for_server_invalid;
use crate::helpers::request::{exec_post, headers};
use crate::helpers::term::print_error;
use crate::run::Pipeline;
use crate::types::{BldCommand, PushInfo, Result};
use clap::{App, Arg, ArgMatches, SubCommand};
use std::collections::HashSet;

static PUSH: &str = "push";
static PIPELINE: &str = "pipeline";
static SERVER: &str = "server";

pub struct PushCommand;

impl PushCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }

    fn build_payload(name: String) -> Result<HashSet<(String, String)>> {
        let src = Pipeline::read(&name)?;
        let pipeline = Pipeline::parse(&src)?;
        let mut set = HashSet::new();
        set.insert((name, src));
        for step in pipeline.steps.iter() {
            if let Some(pipeline) = &step.call {
                let subset = Self::build_payload(pipeline.to_string())?;
                for entry in subset {
                    set.insert(entry);
                }
            }
        }
        Ok(set)
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

    fn exec(&self, matches: &ArgMatches<'_>) -> Result<()> {
        let config = BldConfig::load()?;
        let pip = matches
            .value_of(PIPELINE)
            .or(Some(TOOL_DEFAULT_PIPELINE))
            .unwrap()
            .to_string();
        let srv = config.remote.server_or_first(matches.value_of(SERVER))?;
        let (name, auth) = match &srv.same_auth_as {
            Some(name) => match config.remote.servers.iter().find(|s| &s.name == name) {
                Some(srv) => (&srv.name, &srv.auth),
                None => return auth_for_server_invalid(),
            },
            None => (&srv.name, &srv.auth),
        };
        match Self::build_payload(pip) {
            Ok(payload) => {
                let sys = String::from("bld-push");
                let data: Vec<PushInfo> = payload
                    .iter()
                    .map(|(n, s)| {
                        println!("Pushing {}...", n);
                        PushInfo::new(n, s)
                    })
                    .collect();
                let url = format!("http://{}:{}/push", srv.host, srv.port);
                let headers = headers(name, auth)?;
                exec_post(sys, url, headers, data);
            }
            Err(e) => {
                let _ = print_error(&e.to_string());
            }
        }
        Ok(())
    }
}
