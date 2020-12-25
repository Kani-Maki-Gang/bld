use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let host = Arg::with_name("host")
        .long("host")
        .help("the server's host address")
        .takes_value(true);

    let port = Arg::with_name("port")
        .long("port")
        .help("the server's port")
        .takes_value(true);

    SubCommand::with_name("server")
        .about("Start bld in server mode, listening to incoming build requests")
        .version(VERSION)
        .args(&vec![host, port])
}
