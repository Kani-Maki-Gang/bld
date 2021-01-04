use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let pipeline = Arg::with_name("pipeline")
        .short("p")
        .long("pipeline")
        .help("the name of the pipeline to push")
        .takes_value(true);
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .help("the name of the server to push changes to")
        .takes_value(true);
    SubCommand::with_name("push")
        .about("Pushes the contents of a pipeline to a bld server")
        .version(VERSION)
        .args(&vec![pipeline, server])
}
