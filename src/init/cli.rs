use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .help("initialize configuration for a bld server");

    SubCommand::with_name("init")
        .about("Initializes the build configuration")
        .version(VERSION)
        .arg(server)
}
