use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let pipeline = Arg::with_name("pipeline")
        .short("p")
        .long("pipeline")
        .help("path to pipeline script")
        .takes_value(true);
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .help("the name of the server to run the pipeline")
        .takes_value(true);
    let detach = Arg::with_name("detach")
        .short("d")
        .long("detach")
        .help("detaches from the run execution (for server mode runs)");
    let variables = Arg::with_name("variables")
        .short("v")
        .long("variables")
        .help("define values for variables of a pipeline")
        .multiple(true)
        .takes_value(true);
    SubCommand::with_name("run")
        .about("Executes a build pipeline")
        .version(VERSION)
        .args(&[pipeline, server, detach, variables])
}
