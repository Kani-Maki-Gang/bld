use anyhow::{anyhow, bail, Result};
use bld_config::{
    definitions::{LOCAL_MACHINE_TMP_DIR, TOOL_DEFAULT_CONFIG_FILE, TOOL_DIR},
    path, BldConfig,
};
use bld_utils::{fs::IsYaml, sync::IntoArc};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    sqlite::SqliteConnection,
};
use std::{
    env::current_dir,
    fmt::Write as FmtWrite,
    fs::{copy, create_dir_all, read_to_string, remove_file, rename, File},
    io::Write,
    path::PathBuf,
    process::{Command, ExitStatus},
    sync::Arc,
};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::database::pipeline::{self, Pipeline};

pub enum PipelineFileSystemProxy {
    Local {
        config: Arc<BldConfig>,
    },
    Server {
        config: Arc<BldConfig>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    },
}

impl Default for PipelineFileSystemProxy {
    fn default() -> Self {
        Self::Local {
            config: BldConfig::default().into_arc(),
        }
    }
}

impl PipelineFileSystemProxy {
    pub fn local(config: Arc<BldConfig>) -> Self {
        Self::Local { config }
    }

    pub fn server(
        config: Arc<BldConfig>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    ) -> Self {
        Self::Server { config, pool }
    }

    pub fn path(&self, name: &str) -> Result<PathBuf> {
        match self {
            Self::Local { .. } => Ok(path![current_dir()?, TOOL_DIR, name]),
            Self::Server { config, pool } => {
                let mut conn = pool.get()?;
                let pip = pipeline::select_by_name(&mut conn, name)?;
                Ok(path![
                    &config.local.server.pipelines,
                    format!("{}.yaml", pip.id)
                ])
            }
        }
    }

    fn pipeline_path(&self, pipeline: &Pipeline) -> Result<PathBuf> {
        let Self::Server { config, .. } = self else {
            bail!("pipeline path isn't supported for a local proxy");
        };
        Ok(path![
            &config.local.server.pipelines,
            format!("{}.yaml", pipeline.id)
        ])
    }

    pub fn tmp_path(&self, name: &str) -> Result<PathBuf> {
        Ok(path![current_dir()?, LOCAL_MACHINE_TMP_DIR, name])
    }

    fn read_internal(&self, path: &PathBuf) -> Result<String> {
        if path.is_yaml() {
            read_to_string(path).map_err(|e| anyhow!(e))
        } else {
            bail!("pipeline not found")
        }
    }

    pub fn read(&self, name: &str) -> Result<String> {
        let path = self.path(name)?;
        self.read_internal(&path)
    }

    pub fn read_tmp(&self, name: &str) -> Result<String> {
        let path = self.tmp_path(name)?;
        self.read_internal(&path)
    }

    fn create_internal(&self, path: &PathBuf, content: &str, overwrite: bool) -> Result<()> {
        if path.is_yaml() && !overwrite {
            bail!("pipeline already exists");
        } else if path.is_yaml() && overwrite {
            remove_file(path)?;
        }

        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }

        let mut handle = File::create(path)?;
        handle.write_all(content.as_bytes())?;

        Ok(())
    }

    pub fn create(&self, name: &str, content: &str, overwrite: bool) -> Result<()> {
        if let Self::Server { pool, .. } = self {
            let mut conn = pool.get()?;
            let response = pipeline::select_by_name(&mut conn, name);
            if response.is_err() {
                let id = Uuid::new_v4().to_string();
                pipeline::insert(&mut conn, &id, name)?;
            }
        }

        let path = self.path(name)?;

        self.create_internal(&path, content, overwrite)
    }

    pub fn create_tmp(&self, name: &str, content: &str, overwrite: bool) -> Result<String> {
        let path = self.tmp_path(name)?;
        self.create_internal(&path, content, overwrite)?;
        Ok(path.display().to_string())
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let path = self.path(name)?;
        match self {
            Self::Local { .. } => {
                if path.is_yaml() {
                    remove_file(&path)?;
                    Ok(())
                } else {
                    bail!("pipeline not found")
                }
            }
            Self::Server { pool, .. } => {
                if path.is_yaml() {
                    let mut conn = pool.get()?;
                    pipeline::delete_by_name(&mut conn, name)
                        .and_then(|_| remove_file(path).map_err(|e| anyhow!(e)))
                        .map_err(|_| anyhow!("unable to remove pipeline"))
                } else {
                    bail!("pipeline not found")
                }
            }
        }
    }

    pub fn remove_tmp(&self, name: &str) -> Result<()> {
        let path = self.tmp_path(name)?;
        if path.is_yaml() {
            remove_file(path)?;
            Ok(())
        } else {
            bail!("pipeline not found");
        }
    }

    pub fn copy(&self, source: &str, target: &str) -> Result<()> {
        let source_path = self.path(source)?;
        if !source_path.is_yaml() {
            bail!("invalid source pipeline path");
        }

        let target_path = self.path(target)?;
        if !target_path.valid_path() {
            bail!("invalid target pipeline path");
        }

        match self {
            Self::Local { .. } => {
                copy(source_path, target_path)?;
                Ok(())
            }
            Self::Server { .. } => {
                let content = self.read(source)?;
                self.create(target, &content, false)
            }
        }
    }

    pub fn mv(&self, source: &str, target: &str) -> Result<()> {
        let source_path = self.path(source)?;
        if !source_path.is_yaml() {
            bail!("invalid source pipeline path");
        }

        let target_path = self.path(target)?;
        if !target_path.valid_path() {
            bail!("invalid target pipeline path");
        }

        match self {
            Self::Local { .. } => {
                rename(source_path, target_path)?;
                Ok(())
            }
            Self::Server { pool, .. } => {
                let mut conn = pool.get()?;
                let source_pipeline = pipeline::select_by_name(&mut conn, source)?;
                if pipeline::select_by_name(&mut conn, target).is_ok() {
                    bail!("target pipeline already exist");
                };
                pipeline::update_name(&mut conn, &source_pipeline.id, target)
            }
        }
    }

    pub fn list(&self) -> Result<Vec<String>> {
        match self {
            Self::Local { .. } => {
                let root = path![current_dir()?, TOOL_DIR];
                let root_str = format!("{}/", root.display());
                let entries = WalkDir::new(root)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_yaml())
                    .map(|e| {
                        let mut entry = e.path().display().to_string();
                        entry = entry.replace(&root_str, "");
                        entry
                    })
                    .collect();
                Ok(entries)
            }
            Self::Server { pool, .. } => {
                let mut conn = pool.get()?;
                let pips = pipeline::select_all(&mut conn)?
                    .iter()
                    .filter(|p| {
                        self.pipeline_path(p)
                            .as_ref()
                            .map(|p| p.is_yaml())
                            .unwrap_or_default()
                    })
                    .map(|p| p.name.clone())
                    .collect();
                Ok(pips)
            }
        }
    }

    fn edit_internal(&self, path: &PathBuf, check_path: bool) -> Result<()> {
        let Self::Local {config} = self else {
            bail!("server pipelines dont support direct editing");
        };

        if check_path && !path.is_yaml() {
            bail!("pipeline not found");
        }

        let mut editor = Command::new("/bin/bash");
        editor.args(["-c", &format!("{} {}", config.local.editor, path.display())]);

        let status = editor.status()?;
        if !ExitStatus::success(&status) {
            let mut error = String::new();
            let output = editor.output()?;
            writeln!(error, "editor process finished with {}", status)?;
            write!(error, "{}", String::from_utf8_lossy(&output.stderr))?;
            bail!(error);
        }
        Ok(())
    }

    pub fn edit(&self, name: &str) -> Result<()> {
        let path = self.path(name)?;
        self.edit_internal(&path, true)
    }

    pub fn edit_tmp(&self, name: &str) -> Result<()> {
        let path = self.tmp_path(name)?;
        self.edit_internal(&path, true)
    }

    pub fn edit_config(&self) -> Result<()> {
        let config_path = path![current_dir()?, TOOL_DIR, TOOL_DEFAULT_CONFIG_FILE];
        self.edit_internal(&config_path, false)
    }
}
