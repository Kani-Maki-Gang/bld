use super::builder::RunnerBuilder;
use crate::pipeline::external::ExternalV1;
use crate::pipeline::step::{BuildStepExecV1, BuildStepV1};
use crate::pipeline::PipelineV1;
use crate::platform::TargetPlatform;
use actix::{io::SinkWrite, Actor, StreamHandler};
use anyhow::{anyhow, bail, Result};
use bld_config::definitions::{
    ENV_TOKEN, GET, PUSH, RUN_PROPS_ID, RUN_PROPS_START_TIME, VAR_TOKEN,
};
use bld_config::BldConfig;
use bld_core::context::ContextSender;
use bld_core::execution::Execution;
use bld_core::logger::LoggerSender;
use bld_core::proxies::PipelineFileSystemProxy;
use bld_sock::clients::ExecClient;
use bld_sock::messages::{RunInfo, WorkerMessages};
use bld_utils::request::WebSocket;
use bld_utils::sync::IntoArc;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::fmt::Write;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tracing::debug;

type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

pub enum VersionedRunner {
    Version1(RunnerV1),
}

impl VersionedRunner {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Version1(runner) => runner.run().await.await,
        }
    }
}

pub struct RunnerV1 {
    pub run_id: String,
    pub run_start_time: String,
    pub config: Arc<BldConfig>,
    pub execution: Arc<Execution>,
    pub logger: Arc<LoggerSender>,
    pub proxy: Arc<PipelineFileSystemProxy>,
    pub pipeline: PipelineV1,
    pub ipc: Arc<Option<Sender<WorkerMessages>>>,
    pub env: Arc<HashMap<String, String>>,
    pub vars: Arc<HashMap<String, String>>,
    pub context: Arc<ContextSender>,
    pub platform: TargetPlatform,
    pub is_child: bool,
    pub has_faulted: bool,
}

impl RunnerV1 {
    async fn register_start(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting the pipeline as running in the execution context");
            self.execution.set_as_running()?;
        }
        Ok(())
    }

    async fn register_completion(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting state of root pipeline");
            if self.has_faulted {
                self.execution.set_as_faulted()?;
            } else {
                self.execution.set_as_finished()?;
            }
        }
        if self.pipeline.dispose {
            debug!("executing dispose operations for platform");
            self.platform.dispose(self.is_child).await?;
        } else {
            debug!("keeping platform alive");
            self.platform.keep_alive().await?;
        }
        Ok(())
    }

    fn check_stop_signal(&self) -> Result<()> {
        debug!("checking for stop signal");
        self.execution.check_stop_signal()
    }

    async fn ipc_send_completed(&self) -> Result<()> {
        if !self.is_child {
            if let Some(ipc) = Option::as_ref(&self.ipc) {
                debug!("sending message to supervisor for a completed run");
                ipc.send(WorkerMessages::Completed).await?;
            }
        }
        Ok(())
    }

    async fn info(&self) -> Result<()> {
        debug!("printing pipeline informantion");

        let mut message = String::new();

        writeln!(message)?;
        if let Some(name) = &self.pipeline.name {
            writeln!(message, "{:<10}: {name}", "Name")?;
        }
        writeln!(message, "{:<10}: {}", "Runs on", &self.pipeline.runs_on)?;
        writeln!(message, "{:<10}: 1", "Version")?;
        writeln!(message)?;

        self.logger.write(message).await
    }

    fn apply_run_properties(&self, txt: &str) -> String {
        let mut txt_with_props = String::from(txt);
        txt_with_props = txt_with_props.replace(RUN_PROPS_ID, &self.run_id);
        txt_with_props = txt_with_props.replace(RUN_PROPS_START_TIME, &self.run_start_time);
        txt_with_props
    }

    fn apply_environment(&self, txt: &str) -> String {
        let mut txt_with_env = String::from(txt);
        for (key, value) in self.env.iter() {
            let full_name = format!("{ENV_TOKEN}{key}");
            txt_with_env = txt_with_env.replace(&full_name, value);
        }

        for (key, value) in self.pipeline.environment.iter() {
            let full_name = format!("{ENV_TOKEN}{}", &key);
            txt_with_env = txt_with_env.replace(&full_name, &value);
        }

        txt_with_env
    }

    fn apply_variables(&self, txt: &str) -> String {
        let mut txt_with_vars = String::from(txt);
        for (key, value) in self.vars.iter() {
            let full_name = format!("{VAR_TOKEN}{key}");
            txt_with_vars = txt_with_vars.replace(&full_name, value);
        }

        for (key, value) in self.pipeline.variables.iter() {
            let full_name = format!("{VAR_TOKEN}{}", &key);
            txt_with_vars = txt_with_vars.replace(&full_name, &value);
        }

        txt_with_vars
    }

    fn apply_context(&self, txt: &str) -> String {
        let txt = self.apply_run_properties(txt);
        let txt = self.apply_environment(&txt);
        self.apply_variables(&txt)
    }

    async fn artifacts(&self, name: &Option<String>) -> Result<()> {
        debug!("executing artifact operation related to step {:?}", name);

        for artifact in self.pipeline.artifacts.iter().filter(|a| &a.after == name) {
            let can_continue =
                artifact.method == PUSH.to_string() || artifact.method == GET.to_string();

            if can_continue {
                debug!("applying context for artifact");

                let method = self.apply_context(&artifact.method);
                let from = self.apply_context(&artifact.from);
                let to = self.apply_context(&artifact.to);
                self.logger
                    .write_line(format!(
                        "Copying artifacts from: {from} into container to: {to}",
                    ))
                    .await?;

                let result = match &method[..] {
                    PUSH => {
                        debug!("executing {PUSH} artifact operation");
                        self.platform.push(&from, &to).await
                    }
                    GET => {
                        debug!("executing {GET} artifact operation");
                        self.platform.get(&from, &to).await
                    }
                    _ => unreachable!(),
                };

                if !artifact.ignore_errors {
                    result?;
                }
            }
        }

        Ok(())
    }

    async fn steps(&mut self) -> Result<()> {
        debug!("starting execution of pipeline steps");
        for step in &self.pipeline.steps {
            self.step(step).await?;
            self.artifacts(&step.name).await?;
            self.check_stop_signal()?;
        }
        Ok(())
    }

    async fn step(&self, step: &BuildStepV1) -> Result<()> {
        if let Some(name) = &step.name {
            self.logger.write_line(format!("Step: {name}")).await?;
        }

        for exec in &step.exec {
            match exec {
                BuildStepExecV1::Shell(cmd) => self.shell(&step, &cmd).await?,
                BuildStepExecV1::External { value } => self.external(&value.as_ref()).await?,
            }
        }

        Ok(())
    }

    async fn external(&self, value: &str) -> Result<()> {
        debug!("starting execution of external section {value}");

        let Some(external) = self
            .pipeline
            .external
            .iter()
            .find(|i| i.is(value)) else {
                self.local_external(&ExternalV1::local(&value)).await?;
                return Ok(());
            };

        match external.server.as_ref() {
            Some(server) => self.server_external(server, external).await?,
            None => self.local_external(external).await?,
        };

        Ok(())
    }

    async fn local_external(&self, details: &ExternalV1) -> Result<()> {
        debug!("building runner for child pipeline");

        let variables: HashMap<String, String> = details
            .variables
            .iter()
            .map(|(key, value)| (key.to_owned(), self.apply_context(&value)))
            .collect();

        let environment: HashMap<String, String> = details
            .environment
            .iter()
            .map(|(key, value)| (key.to_owned(), self.apply_context(&value)))
            .collect();

        let runner = RunnerBuilder::default()
            .run_id(&self.run_id)
            .run_start_time(&self.run_start_time)
            .config(self.config.clone())
            .proxy(self.proxy.clone())
            .pipeline(&details.pipeline)
            .execution(self.execution.clone())
            .logger(self.logger.clone())
            .environment(environment.into_arc())
            .variables(variables.into_arc())
            .ipc(self.ipc.clone())
            .context(self.context.clone())
            .is_child(true)
            .build()
            .await?;

        debug!("starting child pipeline runner");

        runner.run().await?;
        self.check_stop_signal()?;

        Ok(())
    }

    async fn server_external(&self, server: &str, details: &ExternalV1) -> Result<()> {
        let server = self.config.server(&server)?;
        let server_auth = self.config.same_auth_as(server)?;
        let variables = details
            .variables
            .iter()
            .map(|(key, value)| (key.to_owned(), self.apply_context(&value)))
            .collect();

        let environment = details
            .environment
            .iter()
            .map(|(key, value)| (key.to_owned(), self.apply_context(&value)))
            .collect();

        let url = format!(
            "{}://{}:{}/ws-exec/",
            server.ws_protocol(),
            server.host,
            server.port
        );

        debug!(
            "establishing web socket connection with server {}",
            server.name
        );

        let (_, framed) = WebSocket::new(&url)?
            .auth(server_auth)
            .request()
            .connect()
            .await
            .map_err(|e| anyhow!(e.to_string()))?;
        let (sink, stream) = framed.split();
        let addr = ExecClient::create(|ctx| {
            ExecClient::add_stream(stream, ctx);
            ExecClient::new(self.logger.clone(), SinkWrite::new(sink, ctx))
        });

        debug!("sending message for pipeline execution over the web socket");

        addr.send(RunInfo::new(
            &details.pipeline,
            Some(environment),
            Some(variables),
        ))
        .await
        .map_err(|e| anyhow!(e))?;

        while addr.connected() {
            sleep(Duration::from_millis(300)).await;
        }

        Ok(())
    }

    async fn shell(&self, step: &BuildStepV1, command: &str) -> Result<()> {
        debug!("start execution of exec section for step");
        let working_dir = step.working_dir.as_ref().map(|wd| self.apply_context(wd));
        let command = self.apply_context(command);

        debug!("executing shell command {}", command);
        self.platform
            .shell(&working_dir, &command, self.execution.clone())
            .await?;

        self.check_stop_signal()?;
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        self.register_start().await?;
        self.info().await?;
        Ok(())
    }

    async fn execute(&mut self) -> Result<()> {
        // using let expressions to log the errors and let an empty string be used
        // by the final print_error of main.

        if let Err(e) = self.artifacts(&None).await {
            self.logger.write(e.to_string()).await?;
            self.has_faulted = true;
            bail!("");
        }

        if let Err(e) = self.steps().await {
            self.logger.write(e.to_string()).await?;
            self.has_faulted = true;
            bail!("");
        }

        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        debug!("starting cleanup operations for runner");
        self.register_completion().await?;
        self.ipc_send_completed().await?;
        Ok(())
    }

    pub async fn run(mut self) -> RecursiveFuture {
        Box::pin(async move {
            self.start().await?;
            let execution_result = self.execute().await;
            let cleanup_result = self.cleanup().await;
            debug!("runner completed");
            execution_result.and(cleanup_result)
        })
    }
}
