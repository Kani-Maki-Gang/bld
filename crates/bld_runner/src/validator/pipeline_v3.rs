use crate::{
    pipeline::v3::Pipeline,
    registry::v3::Registry,
    runs_on::v3::RunsOn,
    step::v3::{BuildStep, BuildStepExec},
    traits::Validate,
};
use anyhow::{bail, Result};
use bld_config::{
    definitions::{
        KEYWORD_BLD_DIR_V3, KEYWORD_PROJECT_DIR_V3, KEYWORD_RUN_PROPS_ID_V3,
        KEYWORD_RUN_PROPS_START_TIME_V3,
    },
    DockerUrl,
};
use bld_config::{path, BldConfig, SshUserAuth};
use bld_core::fs::FileSystem;
use bld_utils::fs::IsYaml;
use cron::Schedule;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

pub struct PipelineValidator<'a> {
    pipeline: &'a Pipeline,
    config: Arc<BldConfig>,
    fs: Arc<FileSystem>,
    regex: Regex,
    keywords: HashSet<&'a str>,
    symbols: HashSet<&'a str>,
    errors: String,
}

impl<'a> Validate for PipelineValidator<'a> {
    async fn validate(mut self) -> Result<()> {
        self.validate_runs_on();
        self.validate_cron();
        self.validate_inputs(None, &self.pipeline.inputs);
        self.validate_environment(None, &self.pipeline.environment);
        self.validate_external().await;
        self.validate_artifacts();
        self.validate_jobs().await;

        if !self.errors.is_empty() {
            bail!(self.errors)
        }

        Ok(())
    }
}

impl<'a> PipelineValidator<'a> {
    pub fn new(
        pipeline: &'a Pipeline,
        config: Arc<BldConfig>,
        fs: Arc<FileSystem>,
    ) -> Result<Self> {
        let regex = Regex::new(r"\$\{\{\s*(\b\w+\b)\s*\}\}")?;
        let keywords = Self::prepare_keywords();
        let symbols = Self::prepare_symbols(pipeline);
        let errors = String::new();
        Ok(Self {
            pipeline,
            config,
            fs,
            regex,
            keywords,
            symbols,
            errors,
        })
    }

    fn prepare_keywords() -> HashSet<&'a str> {
        let mut keywords = HashSet::new();
        keywords.insert(KEYWORD_BLD_DIR_V3);
        keywords.insert(KEYWORD_PROJECT_DIR_V3);
        keywords.insert(KEYWORD_RUN_PROPS_ID_V3);
        keywords.insert(KEYWORD_RUN_PROPS_START_TIME_V3);
        keywords
    }

    fn prepare_symbols(pipeline: &'a Pipeline) -> HashSet<&'a str> {
        let mut symbols = HashSet::new();
        symbols.insert(KEYWORD_BLD_DIR_V3);
        symbols.insert(KEYWORD_PROJECT_DIR_V3);
        symbols.insert(KEYWORD_RUN_PROPS_ID_V3);
        symbols.insert(KEYWORD_RUN_PROPS_START_TIME_V3);

        for (k, _) in pipeline.inputs.iter() {
            symbols.insert(k);
        }

        for (k, _) in pipeline.environment.iter() {
            symbols.insert(k);
        }

        symbols
    }

    fn sanitize_symbol(symbol: &'a str) -> &'a str {
        symbol[3..symbol.len() - 2].trim()
    }

    fn validate_keywords(&mut self, section: &str, name: &'a str) {
        if self.keywords.contains(name) {
            let _ = writeln!(self.errors, "[{section}] Invalid name, reserved as keyword",);
        }
    }

    fn validate_symbols(&mut self, section: &str, value: &'a str) {
        for symbol in self.regex.find_iter(value).map(|x| x.as_str()) {
            if !self.symbols.contains(Self::sanitize_symbol(symbol)) {
                let _ = writeln!(
                    self.errors,
                    "[{section} > {symbol}] Expression isn't a keyword or variable",
                );
            }
        }
    }

    fn contains_symbols(&mut self, value: &str) -> bool {
        self.regex.find(value).is_some()
    }

    fn validate_runs_on(&mut self) {
        match &self.pipeline.runs_on {
            RunsOn::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => {
                self.validate_symbols("runs_on > name", name);
                self.validate_symbols("runs_on > tag", tag);
                self.validate_symbols("runs_on > dockerfile", dockerfile);
                self.validate_file_path("runs_on > dockerfile", dockerfile);
                if let Some(docker_url) = docker_url {
                    self.validate_symbols("runs_on > docker_url", docker_url);
                    self.validate_docker_url(docker_url);
                }
            }

            RunsOn::Pull {
                image,
                docker_url,
                pull: _pull,
                registry,
            } => {
                self.validate_symbols("runs_on > image", image);
                if let Some(docker_url) = docker_url {
                    self.validate_symbols("runs_on > docker_url", docker_url);
                    self.validate_docker_url(docker_url);
                }
                if let Some(registry) = registry {
                    self.validate_registry("runs_on > registry", registry);
                }
            }

            RunsOn::ContainerOrMachine(value) => self.validate_symbols("runs_on", value),

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                self.validate_symbols("runs_on > ssh_config", ssh_config);
                self.validate_global_ssh_config("runs_on > ssh_config", ssh_config);
            }

            RunsOn::Ssh(config) => {
                self.validate_symbols("runs_on > host", &config.host);
                self.validate_symbols("runs_on > port", &config.port);
                self.validate_symbols("runs_on > user", &config.user);
                match &config.userauth {
                    SshUserAuth::Agent => {}
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => {
                        if let Some(pubkey) = public_key {
                            self.validate_symbols("runs_on > auth > public_key", pubkey);
                            self.validate_file_path("runs_on > auth > public_key", pubkey);
                        }
                        self.validate_symbols("runs_on > auth > private_key", private_key);
                        self.validate_file_path("runs_on > auth > private_key", private_key);
                    }
                    SshUserAuth::Password { password } => {
                        self.validate_symbols("runs_on > auth > password", password);
                    }
                }
            }
        }
    }

    fn validate_file_path(&mut self, section: &str, value: &str) {
        if self.contains_symbols(value) {
            return;
        }
        let path = path![value];
        if !path.is_file() {
            let _ = writeln!(self.errors, "[{section} > {value}] File not found");
        }
    }

    fn validate_docker_url(&mut self, value: &str) {
        if self.contains_symbols(value) {
            return;
        }
        match &self.config.local.docker_url {
            DockerUrl::Single(_) => {
                let _ = writeln!(
                    self.errors,
                    "[runs_on > docker_url] Only a single docker url is defined in the config file"
                );
            }
            DockerUrl::Multiple(urls) => {
                let url = urls.keys().find(|x| x.as_str() == value);
                if url.is_none() {
                    let _ = writeln!(self.errors, "[runs_on > docker_url] The defined docker url key wasn't found in the config file");
                }
            }
        }
    }

    fn validate_registry(&mut self, section: &str, registry: &'a Registry) {
        match registry {
            Registry::FromConfig(config) => {
                self.validate_symbols(section, config);
                self.validate_global_registry_config(section, config);
            }
            Registry::Full(config) => {
                self.validate_symbols(&format!("{section} > url"), &config.url);
                if let Some(username) = &config.username {
                    self.validate_symbols(&format!("{section} > username"), username);
                }
                if let Some(password) = &config.password {
                    self.validate_symbols(&format!("{section} > password"), password);
                }
            }
        }
    }

    fn validate_global_registry_config(&mut self, section: &str, value: &str) {
        if self.contains_symbols(value) {
            return;
        }
        if self.config.registry(value).is_none() {
            let _ = writeln!(
                self.errors,
                "[{section}] The defined registry key wasn't found in the config file"
            );
        }
    }

    fn validate_global_ssh_config(&mut self, section: &str, value: &str) {
        if self.contains_symbols(value) {
            return;
        }
        if let Err(e) = self.config.ssh(value) {
            let _ = writeln!(self.errors, "[{section}] {e}");
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

    fn validate_inputs(
        &mut self,
        section: Option<&str>,
        inputs: &'a HashMap<String, String>,
    ) {
        for (k, v) in inputs.iter() {
            let section = section.map(|x| format!("{x} > ")).unwrap_or_default();
            let section = format!("{section}inputs > {k}");
            self.validate_keywords(&section, k);
            self.validate_symbols(&section, v);
        }
    }

    fn validate_environment(
        &mut self,
        section: Option<&str>,
        environment: &'a HashMap<String, String>,
    ) {
        for (k, v) in environment.iter() {
            let section = section.map(|x| format!("{x} > ")).unwrap_or_default();
            let section = format!("{section}environment > {k}");
            self.validate_keywords(&section, k);
            self.validate_symbols(&section, v);
        }
    }

    async fn validate_external(&mut self) {
        for entry in &self.pipeline.external {
            self.validate_external_name(entry.name.as_deref());
            self.validate_external_pipeline(&entry.pipeline).await;
            self.validate_external_server(entry.server.as_deref());
            self.validate_inputs(Some("external"), &entry.inputs);
            self.validate_environment(Some("external"), &entry.environment);
        }
    }

    fn validate_external_name(&mut self, name: Option<&'a str>) {
        let Some(name) = name else {
            return;
        };
        self.validate_symbols("external > name", name)
    }

    async fn validate_external_pipeline(&mut self, pipeline: &'a str) {
        self.validate_symbols("external > pipeline", pipeline);

        if self.contains_symbols(pipeline) {
            return;
        }

        match self.fs.path(pipeline).await {
            Ok(path) if !path.is_yaml() => {
                let _ = writeln!(
                    self.errors,
                    "[external > pipeline > {}] Pipeline not found",
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

        if self.contains_symbols(server) {
            return;
        }

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

        if self.contains_symbols(after) {
            return;
        }

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

    async fn validate_jobs(&mut self) {
        for (job, steps) in self.pipeline.jobs.iter() {
            for step in steps.iter() {
                self.validate_step(job, step).await;
            }
        }
    }

    async fn validate_step(&mut self, job: &str, step: &'a BuildStep) {
        let mut section = format!("jobs > {job} > steps");

        match step {
            BuildStep::One(exec) => {
                self.validate_exec(&section, exec).await;
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
                    self.validate_exec(&section, exec).await;
                }
            }
        }
    }

    async fn validate_exec(&mut self, section: &str, step: &'a BuildStepExec) {
        match step {
            BuildStepExec::Shell(value) => {
                self.validate_symbols(section, value);
            }
            BuildStepExec::External { value } => {
                self.validate_exec_ext(section, value).await;
            }
        }
    }

    async fn validate_exec_ext(&mut self, section: &str, value: &'a str) {
        self.validate_symbols(section, value);

        if self.contains_symbols(value) {
            return;
        }

        if self.pipeline.external.iter().any(|e| e.is(value)) {
            return;
        }

        let found_path = self
            .fs
            .path(value)
            .await
            .map(|x| x.is_yaml())
            .unwrap_or_default();

        if !found_path {
            let _ = writeln!(self.errors, "[{section} > ext > {value}] Not found in either the external section or as a local pipeline");
        }
    }
}
