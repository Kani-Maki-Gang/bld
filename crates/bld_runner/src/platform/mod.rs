mod container;
mod machine;

pub use container::*;
pub use machine::*;

use anyhow::Result;
use bld_core::execution::Execution;
use std::sync::Arc;

pub enum TargetPlatform {
    Machine(Box<Machine>),
    Container(Box<Container>),
}

impl TargetPlatform {
    pub async fn push(&self, from: &str, to: &str) -> Result<()> {
        match self {
            Self::Machine(machine) => machine.copy_into(from, to),
            Self::Container(container) => container.copy_into(from, to).await,
        }
    }

    pub async fn get(&self, from: &str, to: &str) -> Result<()> {
        match self {
            Self::Machine(machine) => machine.copy_from(from, to),
            Self::Container(container) => container.copy_from(from, to).await,
        }
    }

    pub async fn shell(
        &self,
        working_dir: &Option<String>,
        command: &str,
        exec: Arc<Execution>,
    ) -> Result<()> {
        match self {
            Self::Machine(machine) => machine.sh(working_dir, command).await,
            Self::Container(container) => container.sh(working_dir, command, exec).await,
        }
    }

    pub async fn keep_alive(&self) -> Result<()> {
        match self {
            Self::Container(container) => container.keep_alive().await,
            _ => Ok(()),
        }
    }

    pub async fn dispose(&self, in_child_runner: bool) -> Result<()> {
        match self {
            // checking if the runner is a child in order to not cleanup the temp dir for the whole run
            Self::Machine(machine) if !in_child_runner => machine.dispose(),
            Self::Machine(_) => Ok(()),
            Self::Container(container) => container.dispose().await,
        }
    }
}
