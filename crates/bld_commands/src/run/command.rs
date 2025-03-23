use crate::command::BldCommand;
use crate::run::adapter::RunBuilder;
use actix::System;
use anyhow::Result;
use bld_config::BldConfig;
use bld_config::definitions::TOOL_DEFAULT_PIPELINE_FILE;
use bld_utils::sync::IntoArc;
use bld_utils::variables::parse_variables;
use clap::Args;

#[derive(Args)]
#[command(about = "Executes a build pipeline")]
pub struct RunCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
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
        short = 'i',
        long = "input",
        help = "Define value for an input variable. Can be used multiple times"
    )]
    inputs: Vec<String>,

    #[arg(
        short = 'e',
        long = "environment",
        help = "Define value for an environment variable. Can be used multiple times"
    )]
    env: Vec<String>,
}

impl BldCommand for RunCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        System::new().block_on(async move {
            let config = BldConfig::load().await?.into_arc();
            let inputs = parse_variables(&self.inputs);
            let env = parse_variables(&self.env);
            let adapter = RunBuilder::new(config, self.pipeline, inputs, env)
                .server(self.server.as_ref())
                .detach(self.detach)
                .build();

            adapter.run().await
        })
    }
}
