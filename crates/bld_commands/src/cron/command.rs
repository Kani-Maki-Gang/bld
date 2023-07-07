use anyhow::Result;
use clap::{Parser, Subcommand};

use super::{
    add::CronAddCommand, cat::CronCatCommand, list::CronListCommand, remove::CronRemoveCommand,
    update::CronUpdateCommand,
};
use crate::command::BldCommand;

#[derive(Subcommand)]
pub enum CronCommands {
    Cat(CronCatCommand),
    Add(CronAddCommand),
    Ls(CronListCommand),
    Update(CronUpdateCommand),
    Rm(CronRemoveCommand),
}

#[derive(Parser)]
#[command(about = "Configure cron jobs for a bld server")]
pub struct CronCommand {
    #[command(subcommand)]
    command: CronCommands,
}

impl CronCommand {
    pub fn invoke(self) -> Result<()> {
        match self.command {
            CronCommands::Cat(cat) => cat.invoke(),
            CronCommands::Ls(list) => list.invoke(),
            CronCommands::Update(update) => update.invoke(),
            CronCommands::Rm(remove) => remove.invoke(),
            CronCommands::Add(add) => add.invoke(),
        }
    }
}
