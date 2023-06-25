use crate::command::BldCommand;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(about = "Upserts a cron job to a server")]
pub struct CronUpsertCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to upsert the cron job to"
    )]
    server: Option<String>,
}

impl BldCommand for CronUpsertCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        Ok(())
    }
}
