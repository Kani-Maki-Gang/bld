use anyhow::anyhow;
use crate::config::definitions::LOCAL_MACHINE_TMP_DIR;
use crate::os::{self, OSname};
use crate::path;
use crate::persist::Logger;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn could_not_spawn_shell() -> anyhow::Result<()> {
    Err(anyhow!("could not spawn shell"))
}

pub struct Machine {
    tmp_dir: String,
    lg: Arc<Mutex<dyn Logger>>,
}

impl Machine {
    pub fn new(lg: Arc<Mutex<dyn Logger>>) -> anyhow::Result<Self> {
        let tmp_path = path![
            std::env::current_dir()?,
            LOCAL_MACHINE_TMP_DIR,
            Uuid::new_v4().to_string()
        ];
        let tmp_dir = tmp_path.display().to_string();
        if !tmp_path.is_dir() {
            std::fs::create_dir_all(tmp_path)?;
        }
        Ok(Self { tmp_dir, lg })
    }

    fn copy(&self, from: &str, to: &str) -> anyhow::Result<()> {
        std::fs::copy(Path::new(from), Path::new(to))?;
        Ok(())
    }

    pub fn copy_from(&self, from: &str, to: &str) -> anyhow::Result<()> {
        self.copy(from, to)
    }

    pub fn copy_into(&self, from: &str, to: &str) -> anyhow::Result<()> {
        self.copy(from, to)
    }

    pub fn sh(&self, working_dir: &Option<String>, input: &str) -> anyhow::Result<()> {
        let mut logger = self.lg.lock().unwrap();
        let os_name = os::name();
        let current_dir = working_dir
            .as_ref()
            .or(Some(&self.tmp_dir))
            .unwrap()
            .to_string();
        let current_dir = if Path::new(&current_dir).is_relative() {
            path![&self.tmp_dir, current_dir].display().to_string()
        } else {
            current_dir
        };
        let (shell, mut args) = match &os_name {
            OSname::Windows => ("powershell.exe", Vec::<&str>::new()),
            OSname::Linux => ("bash", vec!["-c"]),
            OSname::Mac => ("sh", vec!["-c"]),
            OSname::Unknown => return could_not_spawn_shell(),
        };
        args.push(input);

        let mut command = Command::new(shell);
        command.args(&args);
        command.current_dir(current_dir);

        let process = command.output()?;
        let mut output = String::from_utf8_lossy(&process.stderr).to_string();
        output.push_str(&format!("\r\n{}", String::from_utf8_lossy(&process.stdout)));
        logger.dump(&output);

        Ok(())
    }

    pub fn dispose(&self) -> anyhow::Result<()> {
        std::fs::remove_dir_all(&self.tmp_dir)?;
        Ok(())
    }
}
