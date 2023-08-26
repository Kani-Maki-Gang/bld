use crate::logger::LoggerSender;
use anyhow::{bail, Result};
use bld_config::{os_name, path, BldConfig, OSname};
use std::{
    collections::HashMap,
    fmt::Write,
    fs::{copy, create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    sync::Arc,
};

pub struct Machine {
    tmp_dir: String,
    env: HashMap<String, String>,
}

impl Machine {
    pub fn new(
        id: &str,
        config: Arc<BldConfig>,
        pipeline_env: &HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
    ) -> Result<Self> {
        let tmp_path = config.tmp_full_path(id);
        if !tmp_path.is_dir() {
            create_dir_all(&tmp_path)?;
        }
        Ok(Self {
            tmp_dir: tmp_path.display().to_string(),
            env: Self::create_environment(pipeline_env, env),
        })
    }

    fn create_environment(
        pipeline_env: &HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
    ) -> HashMap<String, String> {
        let mut map = HashMap::new();

        for (k, v) in pipeline_env.iter() {
            map.insert(k.to_owned(), v.to_owned());
        }

        for (k, v) in env.iter() {
            map.insert(k.to_owned(), v.to_owned());
        }

        map
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

    pub async fn sh(
        &self,
        logger: Arc<LoggerSender>,
        working_dir: &Option<String>,
        input: &str,
    ) -> Result<()> {
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
        command.envs(&self.env);
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

        logger.write(output).await?;

        if !ExitStatus::success(&process.status) {
            bail!("command finished with {}", process.status);
        }

        Ok(())
    }

    pub fn dispose(&self) -> Result<()> {
        remove_dir_all(&self.tmp_dir)?;
        Ok(())
    }
}
