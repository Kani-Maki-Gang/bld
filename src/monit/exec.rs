use crate::config::{BldConfig, BldServerConfig};
use crate::definitions::TOOL_DEFAULT_PIPELINE;
use crate::term::print_info;
use clap::ArgMatches;
use std::io::{self, Error, ErrorKind};

fn start(pipeline: &str, server: &BldServerConfig) -> io::Result<()> {
    let message = format!(
        "Starting monitor pipeline: {}, server: {} [ {}:{} ]",
        pipeline, server.name, server.host, server.port
    );

    print_info(&message)
}

pub fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let config = BldConfig::load()?;
    let servers = config.remote.servers;

    let pipeline = match matches.value_of("pipeline") {
        Some(pipeline) => pipeline,
        None => TOOL_DEFAULT_PIPELINE,
    };

    let server = match matches.value_of("server") {
        Some(server) => match servers.iter().find(|s| s.name == server) {
            Some(entry) => entry,
            None => return Err(Error::new(ErrorKind::Other, "no server specified")),
        },
        None => match servers.iter().next() {
            Some(entry) => entry,
            None => return Err(Error::new(ErrorKind::Other, "no server specified")),
        },
    };

    start(pipeline, server)
}
