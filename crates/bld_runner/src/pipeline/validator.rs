use super::step::BuildStepExecV1;
use super::PipelineV1;
use anyhow::{bail, Result};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::fs::IsYaml;
use std::fmt::Write;
use std::sync::Arc;

pub struct PipelineValidatorV1<'a> {
    pipeline: &'a PipelineV1,
    config: Arc<BldConfig>,
    proxy: Arc<PipelineFileSystemProxy>,
}

impl<'a> PipelineValidatorV1<'a> {
    pub fn new(
        pipeline: &'a PipelineV1,
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
    ) -> Self {
        Self {
            pipeline,
            config,
            proxy,
        }
    }

    pub fn validate(&self) -> Result<()> {
        let mut errors = String::new();

        if let Err(e) = self.validate_external() {
            write!(errors, "{e}")?;
        }

        if let Err(e) = self.validate_steps() {
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

    fn validate_external(&self) -> Result<()> {
        let mut errors = String::new();

        for entry in &self.pipeline.external {
            if let Err(e) = self.validate_external_pipeline(&entry.pipeline) {
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

    fn validate_external_pipeline(&self, pipeline: &str) -> Result<()> {
        match self.proxy.path(pipeline) {
            Ok(path) if !path.is_yaml() => {
                bail!("[external > pipeline: {}] ", pipeline)
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
                "[external > server: {}] doesn't exist in current config",
                server
            );
        }

        Ok(())
    }

    fn validate_steps(&self) -> Result<()> {
        let mut errors = String::new();

        for step in &self.pipeline.steps {
            for exec in &step.exec {
                if let Err(e) = self.validate_exec(exec) {
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

    fn validate_exec(&self, step: &BuildStepExecV1) -> Result<()> {
        match step {
            BuildStepExecV1::Shell(_) => Ok(()),
            BuildStepExecV1::External { value } => self.validate_exec_ext(value),
        }
    }

    fn validate_exec_ext(&self, value: &str) -> Result<()> {
        if self.pipeline.external.iter().any(|e| e.is(value)) {
            return Ok(());
        }

        let found_path = self
            .proxy
            .path(value)
            .map(|x| x.is_yaml())
            .unwrap_or_default();
        if !found_path {
            bail!("[steps > exec > ext: {value}] not found in either the external section or as a local pipeline");
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
            bail!("[artifacts > after: {after}] not a declared step name");
        }

        Ok(())
    }
}
