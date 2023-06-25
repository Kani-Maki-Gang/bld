use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::command::BldCommand;
use super::{list::CronListCommand, upsert::CronUpsertCommand, remove::CronRemoveCommand};


#[derive(Subcommand)]
pub enum CronCommands {
    Ls(CronListCommand),
    Upsert(CronUpsertCommand),
    Rm(CronRemoveCommand)
}

#[derive(Parser)]
pub struct CronCommand {
    #[command(subcommand)]
    command: CronCommands
}

impl CronCommand {
    pub fn invoke(self) -> Result<()> {
        match self.command {
            CronCommands::Ls(list) => list.invoke(),
            CronCommands::Upsert(upsert) => upsert.invoke(),
            CronCommands::Rm(remove) => remove.invoke(),
        }
    }
}
