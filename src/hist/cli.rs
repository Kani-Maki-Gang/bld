use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .takes_value(true)
        .help("The name of the server from which to fetch execution history");
    SubCommand::with_name("hist")
        .about("Fetches execution history of pipelines on a server")
        .version(VERSION)
        .args(&[server])
}
