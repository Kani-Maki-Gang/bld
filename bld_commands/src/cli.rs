use clap::{App, ArgMatches};

pub trait BldCommand {
    fn id(&self) -> &'static str;

    fn interface(&self) -> App<'static>;

    fn exec(&self, matches: &ArgMatches) -> anyhow::Result<()>;
}
