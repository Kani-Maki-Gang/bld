use crate::config::definitions::TOOL_DEFAULT_PIPELINE;
use crate::persist::{NullExec, ShellLogger};
use crate::run::{self, Runner};
use crate::types::Result;
use clap::ArgMatches;
use tokio::runtime::Runtime;

pub fn exec(matches: &ArgMatches<'_>) -> Result<()> {
    let pipeline = matches
        .value_of("pipeline")
        .or(Some(TOOL_DEFAULT_PIPELINE))
        .unwrap()
        .to_string();
    match matches.value_of("server") {
        Some(server) => run::on_server(pipeline, server.to_string()),
        None => {
            let mut rt = Runtime::new()?;
            rt.block_on(async {
                Runner::from_file(pipeline, NullExec::atom(), ShellLogger::atom(), None)
                    .await
                    .await
            })
        }
    }
}
