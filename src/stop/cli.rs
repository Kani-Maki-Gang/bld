use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let id = Arg::with_name("id")
        .short("i")
        .long("id")
        .help("the id of a pipeline running on a server")
        .required(true)
        .takes_value(true);
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .help("the name of the server that the pipeline is running")
        .takes_value(true);
    SubCommand::with_name("stop")
        .about("Stops a running pipeline on a server")
        .version(VERSION)
        .args(&[id, server])
}
