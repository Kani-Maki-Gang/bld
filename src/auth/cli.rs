use crate::config::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let server = Arg::with_name("server")
        .short("s")
        .long("server")
        .help("The target bld server")
        .takes_value(true);
    SubCommand::with_name("login")
        .about("Initiates the login process for a bld server")
        .version(VERSION)
        .args(&[server])
}
