use std::{collections::HashMap, fmt::Write, pin::Pin, sync::Arc, time::Duration};

use actix::{clock::sleep, io::SinkWrite, spawn, Actor, StreamHandler};
use anyhow::{anyhow, bail, Result};
use bld_config::{
    definitions::{GET, PUSH},
    BldConfig,
};
use bld_core::{
    context::ContextSender,
    logger::LoggerSender,
    platform::{Image, TargetPlatform},
    proxies::PipelineFileSystemProxy,
    signals::{UnixSignalMessage, UnixSignalsReceiver},
};
use bld_sock::{
    clients::ExecClient,
    messages::{ExecClientMessage, WorkerMessages},
};
use bld_utils::{request::WebSocket, sync::IntoArc};
use futures::{Future, StreamExt};
use tokio::sync::mpsc::Sender;
use tracing::debug;

use crate::{
    external::version2::External,
    pipeline::{traits::ApplyTokens, version2::Pipeline},
    platform::{builder::TargetPlatformBuilder, version2::Platform},
    step::version2::{BuildStep, BuildStepExec},
    token_context::version2::PipelineContext,
    RunnerBuilder,
};

type RecursiveFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

pub struct Runner {
    pub run_id: String,
    pub run_start_time: String,
    pub config: Arc<BldConfig>,
    pub signals: Option<UnixSignalsReceiver>,
    pub logger: Arc<LoggerSender>,
    pub proxy: Arc<PipelineFileSystemProxy>,
    pub pipeline: Pipeline,
    pub ipc: Arc<Option<Sender<WorkerMessages>>>,
    pub env: Arc<HashMap<String, String>>,
    pub vars: Arc<HashMap<String, String>>,
    pub context: Arc<ContextSender>,
    pub platform: Option<Arc<TargetPlatform>>,
    pub is_child: bool,
    pub has_faulted: bool,
}

impl Runner {
    async fn register_start(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting the pipeline as running in the execution context");
            self.context
                .set_pipeline_as_running(self.run_id.to_owned())
                .await?;
        }
        Ok(())
    }

    async fn register_completion(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting state of root pipeline");
            if self.has_faulted {
                self.context
                    .set_pipeline_as_faulted(self.run_id.to_owned())
                    .await?;
            } else {
                self.context
                    .set_pipeline_as_finished(self.run_id.to_owned())
                    .await?;
            }
        }
        Ok(())
    }

    fn apply_context(&mut self) -> Result<()> {
        let context = PipelineContext {
            bld_directory: &self.config.path,
            variables: self.vars.clone(),
            environment: self.env.clone(),
            run_id: &self.run_id,
            run_start_time: &self.run_start_time,
        };
        self.pipeline.apply_tokens(&context)?;
        Ok(())
    }

    async fn create_platform(&mut self) -> Result<()> {
        let image = match &self.pipeline.runs_on {
            Platform::Machine => None,
            Platform::Container(image) | Platform::ContainerByPull { image, pull: false } => {
                Some(Image::Use(image.to_owned()))
            }
            Platform::ContainerByPull { image, pull: true } => Some(Image::Pull(image.to_owned())),
            Platform::ContainerByBuild {
                name,
                tag,
                dockerfile,
            } => Some(Image::Build {
                name: name.to_owned(),
                dockerfile: dockerfile.to_owned(),
                tag: tag.to_owned(),
            }),
        };

        let platform = TargetPlatformBuilder::default()
            .run_id(&self.run_id)
            .config(self.config.clone())
            .image(image)
            .environment(self.env.clone())
            .logger(self.logger.clone())
            .context(self.context.clone())
            .build()
            .await?;

        self.context.add_platform(platform.clone()).await?;
        self.platform = Some(platform);
        Ok(())
    }

    async fn dispose_platform(&self) -> Result<()> {
        let Some(platform) = self.platform.as_ref() else {bail!("no platform instance for runner");};
        if self.pipeline.dispose {
            debug!("executing dispose operations for platform");
            platform.dispose(self.is_child).await?;
        } else {
            debug!("keeping platform alive");
            platform.keep_alive().await?;
        }

        self.context.remove_platform(platform.id()).await
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

        if let Some(name) = &self.pipeline.name {
            writeln!(message, "{:<10}: {name}", "Name")?;
        }
        writeln!(message, "{:<10}: {}", "Runs on", &self.pipeline.runs_on)?;
        writeln!(message, "{:<10}: 2", "Version")?;

        self.logger.write_line(message).await
    }

    async fn artifacts(&self, name: &Option<String>) -> Result<()> {
        debug!("executing artifact operation related to step {:?}", name);

        let Some(platform) = self.platform.as_ref() else {bail!("no platform instance for runner");};

        for artifact in self.pipeline.artifacts.iter().filter(|a| &a.after == name) {
            let can_continue = artifact.method == *PUSH || artifact.method == *GET;

            if can_continue {
                self.logger
                    .write_line(format!(
                        "Copying artifacts from: {} into container to: {}",
                        artifact.from, artifact.to
                    ))
                    .await?;

                let result = match &artifact.method[..] {
                    PUSH => {
                        debug!("executing {PUSH} artifact operation");
                        platform.push(&artifact.from, &artifact.to).await
                    }
                    GET => {
                        debug!("executing {GET} artifact operation");
                        platform.get(&artifact.from, &artifact.to).await
                    }
                    _ => unreachable!(),
                };

                if !artifact.ignore_errors.unwrap_or_default() {
                    result?;
                }
            }
        }

        Ok(())
    }

    async fn steps(&mut self) -> Result<()> {
        self.artifacts(&None).await?;

        debug!("starting execution of pipeline steps");
        for step in &self.pipeline.steps {
            self.step(step).await?;
            self.artifacts(&step.name).await?;
        }

        Ok(())
    }

    async fn step(&self, step: &BuildStep) -> Result<()> {
        if let Some(name) = &step.name {
            let mut message = String::new();
            writeln!(message, "{:<10}: {name}", "Step")?;
            self.logger.write_line(message).await?;
        }

        for exec in &step.exec {
            match exec {
                BuildStepExec::Shell(cmd) => self.shell(step, cmd).await?,
                BuildStepExec::External { value } => self.external(value.as_ref()).await?,
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
                self.local_external(&External::local(value)).await?;
                return Ok(());
            };

        match external.server.as_ref() {
            Some(server) => self.server_external(server, external).await?,
            None => self.local_external(external).await?,
        };

        Ok(())
    }

    async fn local_external(&self, details: &External) -> Result<()> {
        debug!("building runner for child pipeline");

        let variables = details.variables.clone();
        let environment = details.environment.clone();

        let runner = RunnerBuilder::default()
            .run_id(&self.run_id)
            .run_start_time(&self.run_start_time)
            .config(self.config.clone())
            .proxy(self.proxy.clone())
            .pipeline(&details.pipeline)
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

        Ok(())
    }

    async fn server_external(&self, server: &str, details: &External) -> Result<()> {
        let server_name = server.to_owned();
        let server = self.config.server(server)?;
        let server_auth = self.config.same_auth_as(server)?;
        let variables = details.variables.clone();
        let environment = details.environment.clone();

        let url = format!("{}/ws-exec/", server.base_url_ws());

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
            ExecClient::new(
                server_name,
                self.logger.clone(),
                self.context.clone(),
                SinkWrite::new(sink, ctx),
            )
        });

        debug!("sending message for pipeline execution over the web socket");

        addr.send(ExecClientMessage::EnqueueRun {
            name: details.pipeline.to_owned(),
            environment: Some(environment),
            variables: Some(variables),
        })
        .await
        .map_err(|e| anyhow!(e))?;

        while addr.connected() {
            sleep(Duration::from_millis(200)).await;
        }

        Ok(())
    }

    async fn shell(&self, step: &BuildStep, command: &str) -> Result<()> {
        debug!("start execution of exec section for step");
        let Some(platform) = self.platform.as_ref() else {bail!("no platform instance for runner");};

        debug!("executing shell command {}", command);
        platform.shell(&step.working_dir, command).await?;

        Ok(())
    }

    async fn start(&mut self) -> Result<()> {
        self.apply_context()?;
        self.create_platform().await?;
        self.register_start().await?;
        self.info().await?;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        debug!("starting cleanup operations for runner");
        self.register_completion().await?;
        self.dispose_platform().await?;
        self.ipc_send_completed().await?;
        Ok(())
    }

    async fn execute(mut self) -> Result<()> {
        self.start().await?;

        // using let expression to log the errors and let an empty string be used
        // by the final print_error of main.

        let result = if let Err(e) = self.steps().await {
            self.logger.write(e.to_string()).await?;
            self.has_faulted = true;
            Err(anyhow!(""))
        } else {
            Ok(())
        };

        self.stop().await?;

        result
    }

    pub async fn run(mut self) -> RecursiveFuture {
        Box::pin(async move {
            // Changing the value internally since the signals needs to be mutated
            // and child runners wont handle any unix signals.
            let signals = self.signals;
            self.signals = None;

            if self.is_child || signals.is_none() {
                return self.execute().await.map(|_| ());
            }

            let context = self.context.clone();
            let logger = self.logger.clone();
            let mut signals = signals.unwrap();
            let runner_handle = spawn(self.execute());

            loop {
                sleep(Duration::from_millis(200)).await;

                if runner_handle.is_finished() {
                    break runner_handle.await?.map(|_| ());
                }

                if let Ok(signal) = signals.try_next() {
                    match signal {
                        UnixSignalMessage::SIGINT | UnixSignalMessage::SIGTERM => {
                            runner_handle.abort();
                            logger
                                .write_line(
                                    "Runner interruped. Starting graceful shutdown...".to_owned(),
                                )
                                .await?;
                            break context.cleanup().await;
                        }
                    }
                }
            }
        })
    }
}
