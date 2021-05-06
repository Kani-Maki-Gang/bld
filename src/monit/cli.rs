use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let pipeline_id = Arg::with_name("pipeline-id")
        .short("i")
        .long("pipeline-id")
        .help("The id of the pipeline to monitor. Takes precedence over pipeline")
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
    let last = Arg::with_name("last")
        .long("last")
        .help("Monitor the execution of the last invoked pipeline. Takes precedence over pipeline-id and pipeline")
        .takes_value(false);
    SubCommand::with_name("monit")
        .about("Connects to a bld server to monitor the execution of a pipeline")
        .version(VERSION)
        .args(&vec![pipeline_id, pipeline, server, last])
}
