use crate::definitions::TOOL_DEFAULT_PIPELINE;
use crate::run;
use clap::ArgMatches;
use std::io;

pub async fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let pipeline = match matches.value_of("pipeline") {
        Some(name) => name.to_string(),
        None => TOOL_DEFAULT_PIPELINE.to_string(),
    };

    match matches.value_of("server") {
        Some(server) => run::connect(server, &pipeline),
        None => run::sync_from_file(pipeline).await.await,
    }
}
