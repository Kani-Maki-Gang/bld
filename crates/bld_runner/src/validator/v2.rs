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
use cron::Schedule;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::str::FromStr;
use std::sync::Arc;

pub struct PipelineValidator<'a> {
    pipeline: &'a Pipeline,
    config: Arc<BldConfig>,
    proxy: Arc<PipelineFileSystemProxy>,
    regex: Regex,
    symbols: HashSet<&'a str>,
    errors: String,
}

impl<'a> PipelineValidator<'a> {
    pub fn new(
        pipeline: &'a Pipeline,
        config: Arc<BldConfig>,
        proxy: Arc<PipelineFileSystemProxy>,
    ) -> Result<Self> {
        let regex = Regex::new(r"\$\{\{\s*(\b\w+\b)\s*\}\}")?;
        let symbols = Self::prepare_symbols(pipeline);
        let errors = String::new();
        Ok(Self {
            pipeline,
            config,
            proxy,
            regex,
            symbols,
            errors,
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

    pub fn validate(mut self) -> Result<()> {
        self.validate_runs_on();
        self.validate_cron();
        self.validate_variables(None, &self.pipeline.variables);
        self.validate_environment(None, &self.pipeline.environment);
        self.validate_external();
        self.validate_artifacts();
        self.validate_jobs();

        if !self.errors.is_empty() {
            bail!(self.errors)
        }

        Ok(())
    }

    fn sanitize_symbol(symbol: &'a str) -> &'a str {
        symbol[3..symbol.len() - 2].trim()
    }

    fn validate_symbols(&mut self, section: &str, value: &'a str) {
        for symbol in self.regex.find_iter(value).map(|x| x.as_str()) {
            if !self.symbols.contains(Self::sanitize_symbol(symbol)) {
                let _ = writeln!(
                    self.errors,
                    "[{section} > {symbol}] expression isn't a keyword or variable",
                );
            }
        }
    }

    fn validate_runs_on(&mut self) {
        match &self.pipeline.runs_on {
            Platform::Build {
                name,
                tag,
                dockerfile,
            } => {
                self.validate_symbols("runs_on > name", name);
                self.validate_symbols("runs_on > tag", tag);
                self.validate_symbols("runs_on > dockerfile", dockerfile);
            }
            Platform::Pull { image, .. } => self.validate_symbols("runs_on > image", image),
            Platform::ContainerOrMachine(value) => self.validate_symbols("runs_on", value),
        }
    }

    fn validate_cron(&mut self) {
        let Some(cron) = self.pipeline.cron.as_ref() else {
            return;
        };
        if let Err(e) = Schedule::from_str(cron) {
            let _ = writeln!(self.errors, "[cron > {cron}] {e}");
        }
    }

    fn validate_variables(
        &mut self,
        section: Option<&str>,
        variables: &'a HashMap<String, String>,
    ) {
        for (k, v) in variables.iter() {
            let section = section
                .map(|x| format!("{x} > "))
                .unwrap_or_else(String::new);
            let section = format!("{section}variables > {k}");
            self.validate_symbols(&section, v);
        }
    }

    fn validate_environment(
        &mut self,
        section: Option<&str>,
        environment: &'a HashMap<String, String>,
    ) {
        for (k, v) in environment.iter() {
            let section = section
                .map(|x| format!("{x} > "))
                .unwrap_or_else(String::new);
            let section = format!("{section}environment > {k}");
            self.validate_symbols(&section, v);
        }
    }

    fn validate_external(&mut self) {
        for entry in &self.pipeline.external {
            self.validate_external_name(entry.name.as_deref());
            self.validate_external_pipeline(&entry.pipeline);
            self.validate_external_server(entry.server.as_deref());
            self.validate_variables(Some("external"), &entry.variables);
            self.validate_environment(Some("external"), &entry.environment);
        }
    }

    fn validate_external_name(&mut self, name: Option<&'a str>) {
        let Some(name) = name else {
            return;
        };
        self.validate_symbols("external > name", name)
    }

    fn validate_external_pipeline(&mut self, pipeline: &'a str) {
        self.validate_symbols("external > pipeline", pipeline);

        match self.proxy.path(pipeline) {
            Ok(path) if !path.is_yaml() => {
                let _ = writeln!(
                    self.errors,
                    "[external > pipeline > {}] Not found",
                    pipeline
                );
            }
            Err(e) => {
                let _ = writeln!(self.errors, "[external > pipeline > {}] {e}", pipeline);
            }
            _ => {}
        }
    }

    fn validate_external_server(&mut self, server: Option<&'a str>) {
        let Some(server) = server else {
            return;
        };

        self.validate_symbols("external > server", server);

        if self.config.server(server).is_err() {
            let _ = writeln!(
                self.errors,
                "[external > server > {}] Doesn't exist in current config",
                server
            );
        }
    }

    fn validate_artifacts(&mut self) {
        for artifact in self.pipeline.artifacts.iter() {
            self.validate_symbols("artifacts > from", &artifact.from);
            self.validate_symbols("artifacts > to", &artifact.to);
            self.validate_artifact_after(artifact.after.as_ref());
        }
    }

    fn validate_artifact_after(&mut self, after: Option<&'a String>) {
        let Some(after) = after else {
            return;
        };

        self.validate_symbols("artifacts > after", after);

        let is_job_or_step = self
            .pipeline
            .jobs
            .iter()
            .any(|(name, steps)| name == after || steps.iter().any(|s| s.is(after)));

        if !is_job_or_step {
            let _ = writeln!(
                self.errors,
                "[artifacts > after > {after}] Not a declared job or step name"
            );
        }
    }

    fn validate_jobs(&mut self) {
        for (job, steps) in self.pipeline.jobs.iter() {
            for step in steps.iter() {
                self.validate_step(job, step);
            }
        }
    }

    fn validate_step(&mut self, job: &str, step: &'a BuildStep) {
        let mut section = format!("jobs > {job} > steps");

        match step {
            BuildStep::One(exec) => {
                self.validate_exec(&section, exec);
            }
            BuildStep::Many {
                exec,
                working_dir,
                name,
            } => {
                if let Some(name) = name {
                    let _ = write!(section, " > {name}");
                }

                if let Some(wd) = working_dir.as_ref() {
                    self.validate_symbols(&section, wd)
                }

                for exec in exec.iter() {
                    self.validate_exec(&section, exec);
                }
            }
        }
    }

    fn validate_exec(&mut self, section: &str, step: &'a BuildStepExec) {
        match step {
            BuildStepExec::Shell(value) => {
                self.validate_symbols(section, value);
            }
            BuildStepExec::External { value } => {
                self.validate_symbols(section, value);
                self.validate_exec_ext(section, value);
            }
        }
    }

    fn validate_exec_ext(&mut self, section: &str, value: &str) {
        if self.pipeline.external.iter().any(|e| e.is(value)) {
            return;
        }

        let found_path = self
            .proxy
            .path(value)
            .map(|x| x.is_yaml())
            .unwrap_or_default();

        if !found_path {
            let _ = writeln!(self.errors, "[{section} > ext > {value}] Not found in either the external section or as a local pipeline");
        }
    }
}
