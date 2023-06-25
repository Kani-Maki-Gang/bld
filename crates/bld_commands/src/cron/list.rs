use crate::command::BldCommand;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(about = "Lists all registered cron jobs in a server")]
pub struct CronListCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to list the cron jobs from"
    )]
    server: Option<String>,
}

impl BldCommand for CronListCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        Ok(())
    }
}
