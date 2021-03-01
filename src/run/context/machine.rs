use crate::os::{self, OSname};
use crate::persist::Logger;
use crate::types::{BldError, Result};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

fn could_not_spawn_shell() -> Result<()> {
    let message = String::from("could not spawn shell");
    Err(BldError::Other(message))
}

pub struct Machine {
    pub logger: Arc<Mutex<dyn Logger>>,
}

impl Machine {
    pub fn new(logger: Arc<Mutex<dyn Logger>>) -> Result<Self> {
        Ok(Self { logger })
    }

    fn copy(&self, from: &str, to: &str) -> Result<()> {
        std::fs::copy(Path::new(from), Path::new(to))?;
        Ok(())
    }

    pub fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        self.copy(from, to)
    }

    pub fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        self.copy(from, to)
    }

    pub fn sh(&self, working_dir: &Option<String>, input: &str) -> Result<()> {
        let mut logger = self.logger.lock().unwrap();
        let os_name = os::name();
        let (shell, mut args) = match os_name {
            OSname::Windows => ("powershell.exe", Vec::<&str>::new()),
            OSname::Linux => ("bash", vec!["-c"]),
            OSname::Mac => ("sh", vec!["-c"]),
            OSname::Unknown => return could_not_spawn_shell(),
        };
        args.push(input);

        let mut command = Command::new(shell);
        command.args(&args);

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }
        let process = command.output()?;
        let mut output = String::from_utf8_lossy(&process.stderr).to_string();
        output.push_str(&format!("\r\n{}", String::from_utf8_lossy(&process.stdout)));
        logger.dump(&output);

        Ok(())
    }
}
