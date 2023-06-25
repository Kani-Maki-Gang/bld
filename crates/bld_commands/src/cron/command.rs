use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::command::BldCommand;
use super::{list::CronListCommand, upsert::CronUpsertCommand, remove::CronRemoveCommand};


#[derive(Subcommand)]
pub enum CronCommands {
    List(CronListCommand),
    Upsert(CronUpsertCommand),
    Remove(CronRemoveCommand)
}

#[derive(Parser)]
pub struct CronCommand {
    #[command(subcommand)]
    command: CronCommands
}

impl CronCommand {
    pub fn invoke(self) -> Result<()> {
        let cron = Self::parse();

        match cron.command {
            CronCommands::List(list) => list.invoke(),
            CronCommands::Upsert(upsert) => upsert.invoke(),
            CronCommands::Remove(remove) => remove.invoke(),
        }
    }
}
