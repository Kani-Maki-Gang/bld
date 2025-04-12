use std::{fmt::Write, sync::Arc, time::Duration};

use actix::{Actor, StreamHandler, clock::sleep, io::SinkWrite};
use anyhow::{Result, anyhow};
use bld_config::definitions::{GET, PUSH};
use bld_core::logger::Logger;
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
        context::CommonWritableRuntimeExprContext, exec::CommonExprExecutor, parser::EXPR_REGEX,
        traits::EvalExpr,
    },
    external::v3::External,
    step::v3::Step,
};

use super::services::RunServices;

pub struct JobRunner {
    pub job_name: String,
    pub services: Arc<RunServices>,
    pub logger: Arc<Logger>,
    pub expr_wctx: CommonWritableRuntimeExprContext,
}

impl JobRunner {
    pub fn new(job_name: String, services: Arc<RunServices>, logger: Arc<Logger>) -> Self {
        Self {
            job_name,
            services,
            logger,
            expr_wctx: CommonWritableRuntimeExprContext::default(),
        }
    }

    pub async fn run(mut self) -> Result<Self> {
        let pipeline = self.services.pipeline.clone();
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
            .services
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
                        self.services
                            .platform
                            .push(&artifact.from, &artifact.to)
                            .await
                    }
                    GET => {
                        debug!("executing {GET} artifact operation");
                        self.services
                            .platform
                            .get(&artifact.from, &artifact.to)
                            .await
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
            .run_id(&self.services.expr_rctx.run_id)
            .run_start_time(&self.services.expr_rctx.run_start_time)
            .config(self.services.config.clone())
            .fs(self.services.fs.clone())
            .file(&details.uses)
            .logger(self.logger.clone())
            .env(env.into_arc())
            .inputs(inputs.into_arc())
            .context(self.services.run_ctx.clone())
            .platform(self.services.platform.clone())
            .regex_cache(self.services.regex_cache.clone())
            .is_child(true)
            .build()
            .await?;

        debug!("starting child file runner");
        runner.run().await?;

        Ok(())
    }

    async fn server_external(&self, server: &str, details: &External) -> Result<()> {
        let server_name = server.to_owned();
        let server = self.services.config.server(server)?;
        let auth_path = self.services.config.auth_full_path(&server.name);
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
                self.services.run_ctx.clone(),
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

        let regex = Regex::new(EXPR_REGEX)?;
        if let Some(matches) = regex.find(command) {
            let mut command = command.to_string();
            let expr_exec = CommonExprExecutor::new(
                &self.services.pipeline,
                &self.services.expr_rctx,
                &mut self.expr_wctx,
            );
            let matches = matches.as_str();
            let value = expr_exec.eval(matches)?.to_string();
            command = command.replace(matches, &value);

            self.services
                .platform
                .shell(self.logger.clone(), working_dir, &command)
                .await?;
        } else {
            self.services
                .platform
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
