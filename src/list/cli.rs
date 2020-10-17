use crate::definitions::VERSION;
use clap::{App, Arg, SubCommand};

pub fn command() -> App<'static, 'static> {
    let local = Arg::with_name("local") 
        .short("l")
        .long("local")
        .help("list configuration for local options");

    let remote = Arg::with_name("remote")
        .short("r")
        .long("remote")
        .help("list configuration for remote options");

    SubCommand::with_name("ls")
        .about("Lists bld's configuration options")
        .version(VERSION)
        .args(&vec![local, remote])
}