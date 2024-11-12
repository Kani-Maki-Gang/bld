use crate::logger::Logger;
use anyhow::{bail, Result};
use bld_config::{path, BldConfig};
use bld_utils::shell::get_shell;
use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
};
use tokio::fs::{copy, create_dir_all, remove_dir_all};

pub struct Machine {
    tmp_dir: String,
    env: HashMap<String, String>,
}

impl Machine {
    pub async fn new(
        id: &str,
        config: Arc<BldConfig>,
        pipeline_env: &HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
    ) -> Result<Self> {
        let tmp_path = config.tmp_full_path(id);
        if !tmp_path.is_dir() {
            create_dir_all(&tmp_path).await?;
        }
        Ok(Self {
            tmp_dir: tmp_path.display().to_string(),
            env: Self::create_env(pipeline_env, env),
        })
    }

    fn create_env(
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

    async fn copy(&self, from: &str, to: &str) -> Result<()> {
        copy(Path::new(from), Path::new(to)).await?;
        Ok(())
    }

    pub async fn copy_from(&self, from: &str, to: &str) -> Result<()> {
        self.copy(from, to).await
    }

    pub async fn copy_into(&self, from: &str, to: &str) -> Result<()> {
        self.copy(from, to).await
    }

    pub async fn sh(
        &self,
        logger: Arc<Logger>,
        working_dir: &Option<String>,
        input: &str,
    ) -> Result<()> {
        let current_dir = working_dir.as_ref().unwrap_or(&self.tmp_dir).to_string();
        let current_dir = if Path::new(&current_dir).is_relative() {
            path![&self.tmp_dir, current_dir].display().to_string()
        } else {
            current_dir
        };

        let mut shell = get_shell(&mut vec![input])?;
        shell.envs(&self.env);
        shell.current_dir(current_dir);

        let process = shell.output().await?;
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

    pub async fn dispose(&self) -> Result<()> {
        remove_dir_all(&self.tmp_dir).await?;
        Ok(())
    }
}
