use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let pipeline = Arg::with_name("pipeline-id")
        .short("i")
        .long("pipeline-id")
        .help("the name of the pipeline to monitor")
        .takes_value(true);

    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .help("the name of the server to monitor")
        .takes_value(true);

    SubCommand::with_name("monit")
        .about("Connects to a bld server to monitor the execution of a pipeline")
        .version(VERSION)
        .args(&vec![pipeline, server])
}
