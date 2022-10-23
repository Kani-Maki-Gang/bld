use crate::run::invoke::InvokeRun;
use crate::BldCommand;
use anyhow::Result;
use bld_config::definitions::{TOOL_DEFAULT_PIPELINE, VERSION};
use bld_config::BldConfig;
use clap::{Arg, ArgAction, ArgMatches, Command};
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

impl BldCommand for RunCommand {
    fn boxed() -> Box<Self> {
        Box::new(Self)
    }

    fn id(&self) -> &'static str {
        RUN
    }

    fn interface(&self) -> Command {
        let pipeline = Arg::new(PIPELINE)
            .short('p')
            .long(PIPELINE)
            .help("Path to pipeline script")
            .default_value(TOOL_DEFAULT_PIPELINE)
            .action(ArgAction::Set);

        let server = Arg::new(SERVER)
            .short('s')
            .long(SERVER)
            .help("The name of the server to run the pipeline")
            .action(ArgAction::Set);

        let detach = Arg::new(DETACH)
            .short('d')
            .long(DETACH)
            .help("Detaches from the run execution (for server mode runs)")
            .action(ArgAction::SetTrue);

        let variables = Arg::new(VARIABLES)
            .short('v')
            .long(VARIABLES)
            .help("Define values for variables of a pipeline")
            .action(ArgAction::Append);

        let environment = Arg::new(ENVIRONMENT)
            .short('e')
            .long(ENVIRONMENT)
            .help("Define values for environment variables of a pipeline")
            .action(ArgAction::Append);

        Command::new(RUN)
            .about("Executes a build pipeline")
            .version(VERSION)
            .args(&[pipeline, server, detach, variables, environment])
    }

    fn exec(&self, matches: &ArgMatches) -> Result<()> {
        let config = BldConfig::load()?;
        // using an unwrap here because pipeline option has a default value.
        let pipeline = matches.get_one::<String>(PIPELINE).cloned().unwrap();
        let detach = matches.get_flag(DETACH);
        let env = parse_variables(matches, ENVIRONMENT);
        let vars = parse_variables(matches, VARIABLES);
        let server = matches.get_one::<String>(SERVER);

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
        .get_many::<String>(arg)
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
