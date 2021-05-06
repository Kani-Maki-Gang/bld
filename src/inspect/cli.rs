use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let pipeline = Arg::with_name("pipeline")
        .long("pipeline")
        .short("p")
        .help("the of the pipeline to inspect")
        .takes_value(true);
    let server = Arg::with_name("server")
        .long("server")
        .short("s")
        .help("the name of the server from which to inspect the pipeline")
        .takes_value(true);
    SubCommand::with_name("inspect")
        .about("Inspects the contents of a pipeline on a bld server")
        .version(VERSION)
        .args(&[pipeline, server])
}
