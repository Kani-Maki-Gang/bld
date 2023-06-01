use crate::command::BldCommand;
use crate::run::adapter::RunBuilder;
use anyhow::Result;
use bld_config::definitions::TOOL_DEFAULT_PIPELINE_FILE;
use bld_config::BldConfig;
use bld_utils::sync::IntoArc;
use clap::Args;
use std::collections::HashMap;

#[derive(Args)]
#[command(about = "Executes a build pipeline")]
pub struct RunCommand {
    #[arg(short = 'v', long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(short = 'p', long = "pipeline", default_value = TOOL_DEFAULT_PIPELINE_FILE, help = "Path to pipeline script")]
    pipeline: String,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to run the pipeline"
    )]
    server: Option<String>,

    #[arg(
        long = "detach",
        help = "Detaches from the run execution (for server mode runs)"
    )]
    detach: bool,

    #[arg(
        short = 'v',
        long = "variable",
        help = "Define value for a variable. Can be used multiple times"
    )]
    variables: Vec<String>,

    #[arg(
        short = 'e',
        long = "environment",
        help = "Define value for an environment variable. Can be used multiple times"
    )]
    environment: Vec<String>,
}

impl BldCommand for RunCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        let config = BldConfig::load()?.into_arc();
        let variables = parse_variables(&self.variables);
        let environment = parse_variables(&self.environment);
        let adapter = RunBuilder::new(config, self.pipeline, variables, environment)
            .server(self.server.as_ref())
            .detach(self.detach)
            .build();

        adapter.run()
    }
}

pub fn parse_variables(variables: &[String]) -> HashMap<String, String> {
    variables
        .iter()
        .flat_map(|v| {
            let mut split = v.split('=');
            let name = split.next()?.to_owned();
            let value = split.next()?.to_owned();
            Some((name, value))
        })
        .collect::<HashMap<String, String>>()
}
