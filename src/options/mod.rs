use clap::{App, ArgMatches};
use crate::definitions::{VERSION, AUTHOR};

pub fn cli(commands: Vec<App<'static, 'static>>) -> ArgMatches<'static> {
    App::new("Bld")
        .version(VERSION)
        .author(AUTHOR)
        .about("A distributed build tool")
        .subcommands(commands)
        .get_matches()
}
