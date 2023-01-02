use crate::logger::LoggerSender;
use anyhow::{bail, Result};
use bld_config::definitions::LOCAL_MACHINE_TMP_DIR;
use bld_config::{os_name, path, OSname};
use std::collections::HashMap;
use std::env::current_dir;
use std::fmt::Write;
use std::fs::{copy, create_dir_all};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::Arc;

pub struct Machine {
    tmp_dir: String,
    env: Arc<HashMap<String, String>>,
    logger: Arc<LoggerSender>,
}

impl Machine {
    pub fn new(
        id: &str,
        env: Arc<HashMap<String, String>>,
        logger: Arc<LoggerSender>,
    ) -> Result<Self> {
        let tmp_path = path![current_dir()?, LOCAL_MACHINE_TMP_DIR, id];
        let tmp_dir = tmp_path.display().to_string();
        if !tmp_path.is_dir() {
            create_dir_all(tmp_path)?;
        }
        Ok(Self {
            tmp_dir,
            env,
            logger,
        })
    }

    fn copy(&self, from: &str, to: &str) -> Result<()> {
        copy(Path::new(from), Path::new(to))?;
        Ok(())
    }

    pub fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        self.copy(from, to)
    }

    pub fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        self.copy(from, to)
    }

    pub async fn sh(&self, working_dir: &Option<String>, input: &str) -> Result<()> {
        let os_name = os_name();
        let current_dir = working_dir.as_ref().unwrap_or(&self.tmp_dir).to_string();
        let current_dir = if Path::new(&current_dir).is_relative() {
            path![&self.tmp_dir, current_dir].display().to_string()
        } else {
            current_dir
        };

        let (shell, mut args) = match &os_name {
            OSname::Windows => ("powershell.exe", Vec::<&str>::new()),
            OSname::Linux => ("bash", vec!["-c"]),
            OSname::Mac => ("sh", vec!["-c"]),
            OSname::Unknown => bail!("could not spawn shell"),
        };
        args.push(input);

        let mut command = Command::new(shell);
        command.envs(&*self.env);
        command.args(&args);
        command.current_dir(current_dir);

        let process = command.output()?;
        let mut output = String::new();

        if !process.stderr.is_empty() {
            writeln!(output, "{}", String::from_utf8_lossy(&process.stderr))?;
        }

        if !process.stdout.is_empty() {
            writeln!(output, "{}", String::from_utf8_lossy(&process.stdout))?;
        }

        self.logger.write(output).await?;

        if !ExitStatus::success(&process.status) {
            bail!("command finished with {}", process.status);
        }

        Ok(())
    }

    pub fn dispose(&self) -> Result<()> {
        std::fs::remove_dir_all(&self.tmp_dir)?;
        Ok(())
    }
}
