use clap::{App, ArgMatches};

pub trait BldCommand {
    fn id(&self) -> &'static str;

    fn interface(&self) -> App<'static, 'static>;

    fn exec(&self, matches: &ArgMatches<'_>) -> anyhow::Result<()>;
}
