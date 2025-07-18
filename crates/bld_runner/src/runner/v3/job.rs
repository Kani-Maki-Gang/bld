use std::{fmt::Write, sync::Arc, time::Duration};

use actix::{Actor, StreamHandler, clock::sleep, io::SinkWrite};
use anyhow::{Result, anyhow};
use bld_config::{
    BldConfig,
    definitions::{GET, PUSH},
};
use bld_core::{
    context::Context, fs::FileSystem, logger::Logger, platform::Platform, regex::RegexCache,
};
use bld_http::WebSocket;
use bld_models::dtos::ExecClientMessage;
use bld_sock::ExecClient;
use bld_utils::sync::IntoArc;
use futures::StreamExt;
use regex::Regex;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::{
    RunnerBuilder,
    expr::v3::{
        context::{CommonReadonlyRuntimeExprContext, CommonWritableRuntimeExprContext},
        exec::CommonExprExecutor,
        traits::EvalExpr,
    },
    external::v3::External,
    pipeline::v3::Pipeline,
    step::v3::Step,
};

pub struct JobRunner {
    pub job_name: String,
    pub logger: Arc<Logger>,
    pub config: Arc<BldConfig>,
    pub fs: Arc<FileSystem>,
    pub run_ctx: Arc<Context>,
    pub pipeline: Arc<Pipeline>,
    pub platform: Arc<Platform>,
    pub regex_cache: Arc<RegexCache>,
    pub expr_regex: Arc<Regex>,
    pub expr_rctx: Arc<CommonReadonlyRuntimeExprContext>,
    pub expr_wctx: CommonWritableRuntimeExprContext,
}

impl JobRunner {
    pub async fn run(mut self) -> Result<Self> {
        let pipeline = self.pipeline.clone();
        let (_, steps) = pipeline
            .jobs
            .iter()
            .find(|(name, _)| **name == self.job_name)
            .ok_or_else(|| anyhow!("unable to find job with name {}", self.job_name))?;

        self.artifacts(None).await?;

        debug!("starting execution of pipeline steps");
        for step in steps.iter() {
            self.step(step).await?;
        }

        self.artifacts(Some(&self.job_name)).await?;

        Ok(self)
    }

    async fn step(&mut self, step: &Step) -> Result<()> {
        match step {
            Step::SingleSh(sh) => self.shell(&None, sh).await?,

            Step::ComplexSh(complex) => {
                if let Some(name) = complex.name.as_ref() {
                    let mut message = String::new();
                    writeln!(message, "{:<15}: {name}", "Step")?;
                    self.logger.write_line(message).await?;
                }
                self.shell(&complex.working_dir, &complex.run).await?;
                self.artifacts(complex.name.as_deref()).await?;
            }

            Step::ExternalFile(external) => {
                if let Some(name) = external.name.as_ref() {
                    let mut message = String::new();
                    writeln!(message, "{:<15}: {name}", "Step")?;
                    self.logger.write_line(message).await?;
                }
                self.external(external).await?;
            }
        }
        Ok(())
    }

    async fn artifacts(&self, name: Option<&str>) -> Result<()> {
        debug!("executing artifact operation related for {:?}", name);

        for artifact in self
            .pipeline
            .artifacts
            .iter()
            .filter(|a| a.after.as_deref() == name)
        {
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
                        self.platform.push(&artifact.from, &artifact.to).await
                    }
                    GET => {
                        debug!("executing {GET} artifact operation");
                        self.platform.get(&artifact.from, &artifact.to).await
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

    async fn external(&self, external: &External) -> Result<()> {
        debug!("calling external pipeline or action {}", external.uses);

        match external.server.as_ref() {
            Some(server) => self.server_external(server, external).await?,
            None => self.local_external(external).await?,
        };

        Ok(())
    }

    async fn local_external(&self, details: &External) -> Result<()> {
        debug!("building runner for child file");

        let inputs = details.with.clone();
        let env = details.env.clone();

        let runner = RunnerBuilder::default()
            .run_id(&self.expr_rctx.run_id)
            .run_start_time(&self.expr_rctx.run_start_time)
            .config(self.config.clone())
            .fs(self.fs.clone())
            .file(&details.uses)
            .logger(self.logger.clone())
            .env(env.into_arc())
            .inputs(inputs.into_arc())
            .context(self.run_ctx.clone())
            .platform(self.platform.clone())
            .regex_cache(self.regex_cache.clone())
            .is_child(true)
            .build()
            .await?;

        debug!("starting child file runner");
        runner.run().await?;

        Ok(())
    }

    async fn server_external(&self, server: &str, details: &External) -> Result<()> {
        let server_name = server.to_owned();
        let server = self.config.server(server)?;
        let auth_path = self.config.auth_full_path(&server.name);
        let inputs = details.with.clone();
        let env = details.env.clone();

        let url = format!("{}/v1/ws-exec/", server.base_url_ws());

        debug!(
            "establishing web socket connection with server {}",
            server.name
        );

        let (_, framed) = WebSocket::new(&url)?
            .auth(&auth_path)
            .await
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
                self.run_ctx.clone(),
                SinkWrite::new(sink, ctx),
            )
        });

        debug!("sending message for pipeline execution over the web socket");

        addr.send(ExecClientMessage::EnqueueRun {
            name: details.uses.to_owned(),
            env: Some(env),
            inputs: Some(inputs),
        })
        .await
        .map_err(|e| anyhow!(e))?;

        while addr.connected() {
            sleep(Duration::from_millis(200)).await;
        }

        Ok(())
    }

    async fn shell(&mut self, working_dir: &Option<String>, command: &str) -> Result<()> {
        debug!("start execution of exec section for step");
        debug!("executing shell command {}", command);

        if let Some(matches) = self.expr_regex.find(command) {
            let mut command = command.to_string();
            let expr_exec = CommonExprExecutor::new(
                self.pipeline.as_ref(),
                self.expr_rctx.as_ref(),
                &mut self.expr_wctx,
            );
            let matches = matches.as_str();
            let value = expr_exec.eval(matches)?.to_string();
            command = command.replace(matches, &value);

            self.platform
                .shell(self.logger.clone(), working_dir, &command)
                .await?;
        } else {
            self.platform
                .shell(self.logger.clone(), working_dir, command)
                .await?;
        }

        Ok(())
    }
}

pub struct RunningJob {
    pub name: String,
    pub handle: JoinHandle<Result<JobRunner>>,
    pub logger: Arc<Logger>,
}

impl RunningJob {
    pub fn new(name: &str, handle: JoinHandle<Result<JobRunner>>, logger: Arc<Logger>) -> Self {
        Self {
            name: name.to_owned(),
            handle,
            logger,
        }
    }
}
