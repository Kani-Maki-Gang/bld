use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let pipeline_id = Arg::with_name("pipeline-id")
        .short("i")
        .long("pipeline-id")
        .help("The id of the pipeline to monitor. It is prioritized over the pipeline argument")
        .takes_value(true);
    let pipeline = Arg::with_name("pipeline")
        .short("p")
        .long("pipeline")
        .help("The name of the pipeline of which to monitor the last run")
        .takes_value(true);
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .help("The name of the server to monitor")
        .takes_value(true);
    SubCommand::with_name("monit")
        .about("Connects to a bld server to monitor the execution of a pipeline")
        .version(VERSION)
        .args(&vec![pipeline_id, pipeline, server])
}
