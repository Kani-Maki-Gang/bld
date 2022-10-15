use crate::run::invoke::InvokeRun;
use crate::BldCommand;
use anyhow::Result;
use bld_config::definitions::{TOOL_DEFAULT_PIPELINE, VERSION};
use bld_config::BldConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use std::collections::HashMap;
use std::fmt::Write;
use tracing::debug;

const RUN: &str = "run";
const PIPELINE: &str = "pipeline";
const SERVER: &str = "server";
const DETACH: &str = "detach";
const VARIABLES: &str = "variables";
const ENVIRONMENT: &str = "environment";

pub struct RunCommand;

impl RunCommand {
    pub fn boxed() -> Box<dyn BldCommand> {
        Box::new(Self)
    }
}

impl BldCommand for RunCommand {
    fn id(&self) -> &'static str {
        RUN
    }

    fn interface(&self) -> App<'static> {
        let pipeline = Arg::with_name(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("Path to pipeline script")
            .takes_value(true);
        let server = Arg::with_name(SERVER)
            .short('s')
            .long(SERVER)
            .help("The name of the server to run the pipeline")
            .takes_value(true);
        let detach = Arg::with_name(DETACH)
            .short('d')
            .long(DETACH)
            .help("Detaches from the run execution (for server mode runs)");
        let variables = Arg::with_name(VARIABLES)
            .short('v')
            .long(VARIABLES)
            .help("Define values for variables of a pipeline")
            .multiple(true)
            .takes_value(true);
        let environment = Arg::with_name(ENVIRONMENT)
            .short('e')
            .long(ENVIRONMENT)
            .help("Define values for environment variables of a pipeline")
            .multiple(true)
            .takes_value(true);
        SubCommand::with_name(RUN)
            .about("Executes a build pipeline")
            .version(VERSION)
            .args(&[pipeline, server, detach, variables, environment])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        let pipeline = matches
            .value_of(PIPELINE)
            .unwrap_or(TOOL_DEFAULT_PIPELINE)
            .to_string();
        let detach = matches.is_present(DETACH);
        let env = parse_variables(matches, ENVIRONMENT);
        let vars = parse_variables(matches, VARIABLES);
        let server = matches.value_of(SERVER);

        let mut message = format!(
            "running {} subcommand with --pipeline: {}, --variables: {:?}",
            RUN, pipeline, vars
        );

        if let Some(server_name) = server {
            write!(message, ", --server: {}", server_name)?;
        }

        debug!(message);

        InvokeRun::new(config, pipeline, server, vars, env, detach)?.start()
    }
}

pub fn parse_variables(matches: &ArgMatches, arg: &str) -> HashMap<String, String> {
    matches
        .values_of(arg)
        .map(|variable| {
            variable
                .map(|v| {
                    let mut split = v.split('=');
                    let name = split.next().unwrap_or("").to_string();
                    let value = split.next().unwrap_or("").to_string();
                    (name, value)
                })
                .collect::<HashMap<String, String>>()
        })
        .or_else(|| Some(HashMap::new()))
        .unwrap()
}
