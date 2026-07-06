use anyhow::Result;
use clap::{Parser, Subcommand};

use super::{
    download::ArtifactsDownloadCommand, list::ArtifactsListCommand, remove::ArtifactsRemoveCommand,
};
use crate::command::BldCommand;

#[derive(Subcommand)]
pub enum ArtifactsCommands {
    Ls(ArtifactsListCommand),
    Download(ArtifactsDownloadCommand),
    Rm(ArtifactsRemoveCommand),
}

#[derive(Parser)]
#[command(about = "Manage the artifacts of pipeline runs on a server")]
pub struct ArtifactsCommand {
    #[command(subcommand)]
    command: ArtifactsCommands,
}

impl ArtifactsCommand {
    pub fn invoke(self) -> Result<()> {
        match self.command {
            ArtifactsCommands::Ls(list) => list.invoke(),
            ArtifactsCommands::Download(download) => download.invoke(),
            ArtifactsCommands::Rm(remove) => remove.invoke(),
        }
    }
}
