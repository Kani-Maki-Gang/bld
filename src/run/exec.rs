use crate::definitions::TOOL_DEFAULT_PIPELINE;
use crate::run;
use clap::ArgMatches;
use std::io;
use tokio::runtime::Runtime;

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let pipeline = match matches.value_of("pipeline") {
        Some(name) => name.to_string(),
        None => TOOL_DEFAULT_PIPELINE.to_string(),
    };

    match matches.value_of("server") {
        Some(server) => run::on_server(server.to_string(), pipeline),
        None => {
            let mut rt = Runtime::new()?;
            rt.block_on(async { run::from_file(pipeline).await.await })
        }
    }
}
