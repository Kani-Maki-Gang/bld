use crate::pipeline::v2::Pipeline;
use crate::platform::v2::Platform;
use crate::step::v2::{BuildStep, BuildStepExec};
use anyhow::{bail, Result};
use bld_config::definitions::{
    KEYWORD_BLD_DIR_V2, KEYWORD_RUN_PROPS_ID_V2, KEYWORD_RUN_PROPS_START_TIME_V2,
};
use bld_config::BldConfig;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_utils::fs::IsYaml;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::sync::Arc;

pub struct PipelineValidator<'a> {
    pipeline: &'a Pipeline,
    config: Arc<BldConfig>,
    proxy: Arc<PipelineFileSystemProxy>,
    regex: Regex,
    symbols: HashSet<&'a str>,
}

impl<'a> PipelineValidator<'a> {
    pub fn new(
        pipeline: &'a Pipeline,
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
    ) -> Result<Self> {
        let regex = Regex::new(r"\$\{\{\s*(\b\w+\b)\s*\}\}")?;
        let symbols = Self::prepare_symbols(pipeline);
        Ok(Self {
            pipeline,
            config,
            proxy,
            regex,
            symbols,
        })
    }

    fn prepare_symbols(pipeline: &'a Pipeline) -> HashSet<&'a str> {
        let mut symbols = HashSet::new();
        symbols.insert(KEYWORD_BLD_DIR_V2);
        symbols.insert(KEYWORD_RUN_PROPS_ID_V2);
        symbols.insert(KEYWORD_RUN_PROPS_START_TIME_V2);

        for (k, _) in pipeline.variables.iter() {
            symbols.insert(k);
        }

        for (k, _) in pipeline.environment.iter() {
            symbols.insert(k);
        }

        symbols
    }

    pub fn validate(&self) -> Result<()> {
        let mut errors = String::new();

        if let Err(e) = self.validate_runs_on() {
            write!(errors, "{e}")?;
        }

        if let Err(e) = self.validate_variables(None, &self.pipeline.variables) {
            write!(errors, "{e}")?;
        }

        if let Err(e) = self.validate_environment(None, &self.pipeline.environment) {
            write!(errors, "{e}")?;
        }

        if let Err(e) = self.validate_external() {
            write!(errors, "{e}")?;
        }

        if let Err(e) = self.validate_jobs() {
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

    fn sanitize_symbol(symbol: &'a str) -> &'a str {
        symbol[3..symbol.len() - 2].trim()
    }

    fn validate_symbols(&self, section: &'a str, value: &'a str) -> Result<()> {
        let mut errors = String::new();

        for symbol in self.regex.find_iter(value).map(|x| x.as_str()) {
            if !self.symbols.contains(Self::sanitize_symbol(symbol)) {
                writeln!(
                    errors,
                    "[{section} > {symbol}] expression isn't a keyword or variable",
                )?;
            }
        }

        if !errors.is_empty() {
            bail!(errors)
        }

        Ok(())
    }

    fn validate_runs_on(&self) -> Result<()> {
        match &self.pipeline.runs_on {
            Platform::Build {
                name,
                tag,
                dockerfile,
            } => {
                let mut errors = String::new();

                if let Err(e) = self.validate_symbols("runs_on > name", name) {
                    write!(errors, "{e}")?;
                }

                if let Err(e) = self.validate_symbols("runs_on > tag", tag) {
                    write!(errors, "{e}")?;
                }

                if let Err(e) = self.validate_symbols("runs_on > dockerfile", dockerfile) {
                    write!(errors, "{e}")?;
                }

                if !errors.is_empty() {
                    bail!(errors)
                }
            }

            Platform::Pull { image, .. } => self.validate_symbols("runs_on > image", image)?,

            Platform::ContainerOrMachine(value) => self.validate_symbols("runs_on >", value)?,
        }
        Ok(())
    }

    fn validate_variables(
        &self,
        section: Option<&str>,
        variables: &HashMap<String, String>,
    ) -> Result<()> {
        let mut errors = String::new();

        for (k, v) in variables.iter() {
            let section = section
                .map(|x| format!("{x} > "))
                .unwrap_or_else(String::new);
            let section = format!("{section}variables > {k}");
            if let Err(e) = self.validate_symbols(&section, v) {
                write!(errors, "{e}")?;
            }
        }

        if !errors.is_empty() {
            bail!(errors)
        }

        Ok(())
    }

    fn validate_environment(
        &self,
        section: Option<&str>,
        environment: &HashMap<String, String>,
    ) -> Result<()> {
        let mut errors = String::new();

        for (k, v) in environment.iter() {
            let section = section
                .map(|x| format!("{x} > "))
                .unwrap_or_else(String::new);
            let section = format!("{section}environment > {k}");
            if let Err(e) = self.validate_symbols(&section, v) {
                write!(errors, "{e}")?;
            }
        }

        if !errors.is_empty() {
            bail!(errors)
        }

        Ok(())
    }

    fn validate_external(&self) -> Result<()> {
        let mut errors = String::new();

        for entry in &self.pipeline.external {
            if let Err(e) = self.validate_external_name(entry.name.as_deref()) {
                writeln!(errors, "{e}")?;
            }

            if let Err(e) = self.validate_external_pipeline(&entry.pipeline) {
                writeln!(errors, "{e}")?;
            }

            if let Err(e) = self.validate_external_server(entry.server.as_deref()) {
                writeln!(errors, "{e}")?;
            }

            if let Err(e) = self.validate_variables(Some("external"), &entry.variables) {
                writeln!(errors, "{e}")?;
            }

            if let Err(e) = self.validate_environment(Some("external"), &entry.environment) {
                writeln!(errors, "{e}")?;
            }
        }

        if !errors.is_empty() {
            bail!(errors)
        }

        Ok(())
    }

    fn validate_external_name(&self, name: Option<&str>) -> Result<()> {
        let Some(name) = name else {
            return Ok(());
        };
        self.validate_symbols("external > name", name)
    }

    fn validate_external_pipeline(&self, pipeline: &str) -> Result<()> {
        let mut errors = String::new();

        if let Err(e) = self.validate_symbols("external > pipeline", pipeline) {
            write!(errors, "{e}")?;
        }

        match self.proxy.path(pipeline) {
            Ok(path) if !path.is_yaml() => {
                write!(errors, "[external > pipeline: {}] Not found", pipeline)?;
            }
            Err(e) => write!(errors, "[external > pipeline: {}] {e}", pipeline)?,
            _ => {}
        }

        if !errors.is_empty() {
            bail!(errors)
        }

        Ok(())
    }

    fn validate_external_server(&self, server: Option<&str>) -> Result<()> {
        let Some(server) = server else {
            return Ok(());
        };

        let mut errors = String::new();

        if let Err(e) = self.validate_symbols("external > server", server) {
            write!(errors, "{e}")?;
        }

        if self.config.server(server).is_err() {
            write!(
                errors,
                "[external > server: {}] Doesn't exist in current config",
                server
            )?;
        }

        if !errors.is_empty() {
            bail!(errors)
        }

        Ok(())
    }

    fn validate_jobs(&self) -> Result<()> {
        let mut errors = String::new();

        for step in self.pipeline.jobs.iter().flat_map(|(_, steps)| steps) {
            match step {
                BuildStep::One(exec) => {
                    if let Err(e) = self.validate_exec(exec) {
                        writeln!(errors, "{e}")?;
                    }
                }
                BuildStep::Many { exec, .. } => {
                    for exec in exec.iter() {
                        if let Err(e) = self.validate_exec(exec) {
                            writeln!(errors, "{e}")?;
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            bail!(errors)
        }
    }

    fn validate_exec(&self, step: &BuildStepExec) -> Result<()> {
        match step {
            BuildStepExec::Shell(_) => Ok(()),
            BuildStepExec::External { value } => self.validate_exec_ext(value),
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
            bail!("[steps > exec > ext: {value}] Not found in either the external section or as a local pipeline");
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
            .jobs
            .iter()
            .any(|(name, steps)| name == after || steps.iter().any(|s| s.is(after)))
        {
            bail!("[artifacts > after: {after}] Not a declared job or step name");
        }

        Ok(())
    }
}
