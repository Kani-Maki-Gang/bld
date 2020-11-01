use crate::definitions::TOOL_DEFAULT_PIPELINE;
use crate::persist::ShellLogger;
use crate::run::{self, Runner};
use clap::ArgMatches;
use std::io;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let pipeline = match matches.value_of("pipeline") {
        Some(name) => name.to_string(),
        None => TOOL_DEFAULT_PIPELINE.to_string(),
    };

    match matches.value_of("server") {
        Some(server) => run::on_server(pipeline, server.to_string()),
        None => {
            let mut rt = Runtime::new()?;
            let logger = Arc::new(Mutex::new(ShellLogger));
            rt.block_on(async { Runner::from_file(pipeline, logger).await.await })
        }
    }
}
