use crate::command::BldCommand;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
#[command(about = "Removes a registered cron job from a server")]
pub struct CronRemoveCommand {
    #[arg(long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[arg(
        short = 's',
        long = "server",
        help = "The name of the server to remove the cron job from"
    )]
    server: Option<String>,
}

impl BldCommand for CronRemoveCommand {
    fn verbose(&self) -> bool {
        self.verbose
    }

    fn exec(self) -> Result<()> {
        Ok(())
    }
}
