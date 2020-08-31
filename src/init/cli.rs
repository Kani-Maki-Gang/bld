use clap::{App, SubCommand};
use crate::definitions::{VERSION, AUTHOR};

pub fn command() -> App<'static, 'static> {
    SubCommand::with_name("init")
        .about("Initializes the build configuration")
        .version(VERSION)
        .author(AUTHOR)
}