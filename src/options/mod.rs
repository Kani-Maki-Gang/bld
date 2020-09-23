use crate::definitions::VERSION;
use clap::{App, ArgMatches};

pub fn cli(commands: Vec<App<'static, 'static>>) -> ArgMatches<'static> {
    App::new("Bld")
        .version(VERSION)
        .about("A distributed CI/CD")
        .subcommands(commands)
        .get_matches()
}
