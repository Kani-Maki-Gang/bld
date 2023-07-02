use anyhow::Result;
use clap::{Parser, Subcommand};

use super::{
    add::CronAddCommand, list::CronListCommand, remove::CronRemoveCommand,
    update::CronUpdateCommand,
};
use crate::command::BldCommand;

#[derive(Subcommand)]
pub enum CronCommands {
    Add(CronAddCommand),
    Ls(CronListCommand),
    Update(CronUpdateCommand),
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
            CronCommands::Update(update) => update.invoke(),
            CronCommands::Rm(remove) => remove.invoke(),
            CronCommands::Add(add) => add.invoke(),
        }
    }
}
