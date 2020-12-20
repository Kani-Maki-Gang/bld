use crate::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .takes_value(true)
        .help("the name of the server from which to fetch pipeline information");
    SubCommand::with_name("ls")
        .about("Lists information of pipelines in a bld server")
        .version(VERSION)
        .args(&vec![server])
}
