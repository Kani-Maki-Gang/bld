use anyhow::Result;
use clap::{Parser, Subcommand};

use super::{list::CronListCommand, remove::CronRemoveCommand, upsert::CronUpsertCommand};
use crate::command::BldCommand;

#[derive(Subcommand)]
pub enum CronCommands {
    Ls(CronListCommand),
    Upsert(CronUpsertCommand),
    Rm(CronRemoveCommand),
}

#[derive(Parser)]
pub struct CronCommand {
    #[command(subcommand)]
    command: CronCommands,
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
