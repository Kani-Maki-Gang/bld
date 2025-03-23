use crate::pipeline::v1::Pipeline;
use crate::step::v1::BuildStepExec;
use crate::traits::Validate;
use anyhow::{Result, bail};
use bld_config::BldConfig;
use bld_core::fs::FileSystem;
use bld_utils::fs::IsYaml;
use std::fmt::Write;
use std::sync::Arc;

pub struct PipelineValidator<'a> {
    pipeline: &'a Pipeline,
    config: Arc<BldConfig>,
    fs: Arc<FileSystem>,
}

impl Validate for PipelineValidator<'_> {
    async fn validate(self) -> Result<()> {
        let mut errors = String::new();

        if let Err(e) = self.validate_external().await {
            write!(errors, "{e}")?;
        }

        if let Err(e) = self.validate_steps().await {
            write!(errors, "{e}")?;
        }

        if let Err(e) = self.validate_artifacts() {
            write!(errors, "{e}")?;
        }

        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors)
        }
    }
}

impl<'a> PipelineValidator<'a> {
    pub fn new(pipeline: &'a Pipeline, config: Arc<BldConfig>, fs: Arc<FileSystem>) -> Self {
        Self {
            pipeline,
            config,
            fs,
        }
    }

    async fn validate_external(&self) -> Result<()> {
        let mut errors = String::new();

        for entry in &self.pipeline.external {
            if let Err(e) = self.validate_external_pipeline(&entry.pipeline).await {
                writeln!(errors, "{e}")?;
            }

            if let Err(e) = self.validate_external_server(entry.server.as_ref()) {
                writeln!(errors, "{e}")?;
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors)
        }
    }

    async fn validate_external_pipeline(&self, pipeline: &str) -> Result<()> {
        match self.fs.path(pipeline).await {
            Ok(path) if !path.is_yaml() => {
                bail!("[external > pipeline: {}] Not found", pipeline)
            }
            Err(e) => bail!("[external > pipeline: {}] {e}", pipeline),
            _ => Ok(()),
        }
    }

    fn validate_external_server(&self, server: Option<&String>) -> Result<()> {
        let Some(server) = server else {
            return Ok(());
        };

        if self.config.server(server).is_err() {
            bail!(
                "[external > server: {}] Doesn't exist in current config",
                server
            );
        }

        Ok(())
    }

    async fn validate_steps(&self) -> Result<()> {
        let mut errors = String::new();

        for step in &self.pipeline.steps {
            for exec in &step.exec {
                if let Err(e) = self.validate_exec(exec).await {
                    writeln!(errors, "{e}")?;
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors)
        }
    }

    async fn validate_exec(&self, step: &BuildStepExec) -> Result<()> {
        match step {
            BuildStepExec::Shell(_) => Ok(()),
            BuildStepExec::External { value } => self.validate_exec_ext(value).await,
        }
    }

    async fn validate_exec_ext(&self, value: &str) -> Result<()> {
        if self.pipeline.external.iter().any(|e| e.is(value)) {
            return Ok(());
        }

        let found_path = self
            .fs
            .path(value)
            .await
            .map(|x| x.is_yaml())
            .unwrap_or_default();
        if !found_path {
            bail!(
                "[steps > exec > ext: {value}] Not found in either the external section or as a local pipeline"
            );
        }

        Ok(())
    }

    fn validate_artifacts(&self) -> Result<()> {
        let mut errors = String::new();

        for artifact in &self.pipeline.artifacts {
            if let Err(e) = self.validate_artifact_after(artifact.after.as_ref()) {
                writeln!(errors, "{e}")?;
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors)
        }
    }

    fn validate_artifact_after(&self, after: Option<&String>) -> Result<()> {
        let Some(after) = after else {
            return Ok(());
        };

        if !self
            .pipeline
            .steps
            .iter()
            .any(|s| s.name.as_ref().map(|n| n == after).unwrap_or_default())
        {
            bail!("[artifacts > after: {after}] Not a declared step name");
        }

        Ok(())
    }
}
