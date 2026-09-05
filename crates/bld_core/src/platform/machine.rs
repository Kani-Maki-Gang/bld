use crate::logger::Logger;
use anyhow::{Result, anyhow, bail};
use bld_config::{BldConfig, definitions::BLD_OUTPUTS_ENV_VAR_V3, path};
use bld_utils::{shell::get_shell, variables::parse_variables_iter};
use std::{
    collections::HashMap,
    fmt::Write,
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::Arc,
};
use tokio::fs::{copy, create_dir_all, read_dir, read_to_string, remove_dir, remove_dir_all};
use tracing::debug;
use uuid::Uuid;

async fn copy_path(from: &Path, to: &Path) -> Result<()> {
    let metadata = tokio::fs::metadata(from).await?;

    if metadata.is_dir() {
        return copy_dir_recursive(from, to).await;
    }

    let mut to = to.to_path_buf();
    if tokio::fs::metadata(&to)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
    {
        let name = from
            .file_name()
            .ok_or_else(|| anyhow!("unable to get the file name of {from:?}"))?;
        to.push(name);
    }

    if let Some(parent) = to.parent() {
        create_dir_all(parent).await?;
    }

    copy(from, &to).await?;
    Ok(())
}

async fn copy_dir_recursive(from: &Path, to: &Path) -> Result<()> {
    create_dir_all(to).await?;

    let mut entries = read_dir(from).await?;
    while let Some(entry) = entries.next_entry().await? {
        let entry_path = entry.path();
        let target_path = to.join(entry.file_name());
        let file_type = entry.file_type().await?;

        if file_type.is_dir() {
            Box::pin(copy_dir_recursive(&entry_path, &target_path)).await?;
        } else {
            copy(&entry_path, &target_path).await?;
        }
    }

    Ok(())
}

pub struct Machine {
    tmp_dir: String,
    env: HashMap<String, String>,
}

impl Machine {
    pub async fn new(
        run_id: &str,
        platform_id: &str,
        config: Arc<BldConfig>,
        pipeline_env: &HashMap<String, String>,
        env: Arc<HashMap<String, String>>,
    ) -> Result<Self> {
        let tmp_path = config.tmp_full_path(run_id).join(platform_id);
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
        copy_path(Path::new(from), Path::new(to)).await
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
    ) -> Result<HashMap<String, String>> {
        let id = Uuid::new_v4();
        let outputs_file = path![&self.tmp_dir, id.to_string()];

        let current_dir = working_dir.as_ref().unwrap_or(&self.tmp_dir).to_string();
        let current_dir = if Path::new(&current_dir).is_relative() {
            path![&self.tmp_dir, current_dir].display().to_string()
        } else {
            current_dir
        };
        debug!("resolved working directory to {current_dir}");

        let mut shell = get_shell(&mut vec![input])?;
        shell.envs(&self.env);
        shell.env(BLD_OUTPUTS_ENV_VAR_V3, &outputs_file);
        shell.current_dir(current_dir);

        let process = shell.output().await?;
        let mut shell_output = String::new();

        if !process.stderr.is_empty() {
            writeln!(shell_output, "{}", String::from_utf8_lossy(&process.stderr))?;
        }

        if !process.stdout.is_empty() {
            writeln!(shell_output, "{}", String::from_utf8_lossy(&process.stdout))?;
        }

        logger.write(shell_output).await?;

        if !ExitStatus::success(&process.status) {
            bail!("command finished with {}", process.status);
        }

        let mut outputs = HashMap::new();
        if outputs_file.exists() {
            let output_content = read_to_string(&outputs_file).await?;
            outputs = parse_variables_iter(output_content.lines());

            if outputs.is_empty() {
                debug!("the executed command created {} outputs", outputs.len());
            }
        }

        Ok(outputs)
    }

    pub async fn dispose(&self) -> Result<()> {
        remove_dir_all(&self.tmp_dir).await?;

        // The directory of the run is shared with the platforms of the other jobs, so it
        // is only removed by the last platform that is disposed. The removal fails while
        // any other job still holds a directory there, which is the expected outcome.
        if let Some(run_dir) = Path::new(&self.tmp_dir).parent() {
            let _ = remove_dir(run_dir).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Machine, copy_path};
    use bld_config::BldConfig;
    use bld_utils::sync::IntoArc;
    use std::collections::HashMap;
    use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
    use std::path::Path;
    use uuid::Uuid;

    #[tokio::test]
    async fn dispose_of_one_machine_platform_does_not_remove_the_directory_of_another() {
        let config = BldConfig::default().into_arc();
        let run_id = format!("machine-dispose-test-{}", Uuid::new_v4());
        let pipeline_env = HashMap::new();
        let env = HashMap::new().into_arc();

        let first = Machine::new(&run_id, "first", config.clone(), &pipeline_env, env.clone())
            .await
            .unwrap();
        let second = Machine::new(
            &run_id,
            "second",
            config.clone(),
            &pipeline_env,
            env.clone(),
        )
        .await
        .unwrap();

        let run_dir = config.tmp_full_path(&run_id);
        let second_marker = Path::new(&second.tmp_dir).join("marker.txt");
        write(&second_marker, b"still here").unwrap();

        first.dispose().await.unwrap();

        assert!(!Path::new(&first.tmp_dir).exists());
        assert!(second_marker.exists());
        assert!(run_dir.is_dir());

        second.dispose().await.unwrap();
        assert!(!Path::new(&second.tmp_dir).exists());
        assert!(!run_dir.exists());
    }

    #[tokio::test]
    async fn copy_path_copies_file_into_directory_target() {
        let config = BldConfig::default();
        let base = config.tmp_full_path(&format!("machine-copy-test-{}", Uuid::new_v4()));
        let source = base.join("file.txt");
        let target_dir = base.join("target");
        create_dir_all(&base).unwrap();
        create_dir_all(&target_dir).unwrap();
        write(&source, b"hello world").unwrap();

        copy_path(&source, &target_dir).await.unwrap();

        let content = read_to_string(target_dir.join("file.txt")).unwrap();
        assert_eq!(content, "hello world");

        let _ = remove_dir_all(&base);
    }

    #[tokio::test]
    async fn copy_path_copies_directory_tree() {
        let config = BldConfig::default();
        let base = config.tmp_full_path(&format!("machine-copy-test-{}", Uuid::new_v4()));
        let source = base.join("source");
        let nested = source.join("nested");
        let target = base.join("target");
        create_dir_all(&nested).unwrap();
        write(source.join("root.txt"), b"root file").unwrap();
        write(nested.join("nested.txt"), b"nested file").unwrap();

        copy_path(&source, &target).await.unwrap();

        let root_content = read_to_string(target.join("root.txt")).unwrap();
        let nested_content = read_to_string(target.join("nested").join("nested.txt")).unwrap();
        assert_eq!(root_content, "root file");
        assert_eq!(nested_content, "nested file");

        let _ = remove_dir_all(&base);
    }
}
