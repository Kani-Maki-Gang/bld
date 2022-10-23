use clap::{ArgMatches, Command};

pub trait BldCommand {
    fn id(&self) -> &'static str;

    fn interface(&self) -> Command;

    fn exec(&self, matches: &ArgMatches) -> anyhow::Result<()>;
}
