use std::{collections::HashMap, fmt::Write, sync::Arc, time::Duration};

use actix::{clock::sleep, spawn};
use anyhow::{anyhow, bail, Result};
use bld_config::{BldConfig, SshUserAuth};
use bld_core::{
    context::Context,
    fs::FileSystem,
    logger::Logger,
    platform::{
        builder::{PlatformBuilder, PlatformOptions},
        Image, Platform, SshAuthOptions, SshConnectOptions,
    },
    regex::RegexCache,
    signals::{UnixSignal, UnixSignalMessage, UnixSignalsBackend},
};
use bld_models::dtos::WorkerMessages;
use bld_utils::sync::IntoArc;
use tokio::sync::mpsc::Sender;
use tracing::debug;

use crate::{
    expr::v3::common::CommonRuntimeExecutionContext, pipeline::v3::Pipeline,
    registry::v3::Registry, runs_on::v3::RunsOn,
};

use super::{
    common::RecursiveFuture,
    job::{JobRunner, RunningJob},
};

pub struct PipelineRunner {
    pub run_id: String,
    pub run_start_time: String,
    pub config: Arc<BldConfig>,
    pub signals: Option<UnixSignalsBackend>,
    pub logger: Arc<Logger>,
    pub regex_cache: Arc<RegexCache>,
    pub fs: Arc<FileSystem>,
    pub pipeline: Arc<Pipeline>,
    pub ipc: Arc<Option<Sender<WorkerMessages>>>,
    pub env: Arc<HashMap<String, String>>,
    pub context: Arc<Context>,
    pub platform: Option<Arc<Platform>>,
    pub is_child: bool,
    pub has_faulted: bool,
}

impl PipelineRunner {
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

    async fn create_platform(&mut self) -> Result<()> {
        let options = match &self.pipeline.runs_on {
            RunsOn::ContainerOrMachine(image) if image == "machine" => PlatformOptions::Machine,

            RunsOn::ContainerOrMachine(image) => PlatformOptions::Container {
                image: Image::Use(image),
                docker_url: None,
            },

            RunsOn::Pull {
                image,
                pull,
                docker_url,
                registry,
            } => {
                let image = if pull.unwrap_or_default() {
                    match registry.as_ref() {
                        Some(Registry::FromConfig(value)) => {
                            Image::pull(image, self.config.registry(value))
                        }
                        Some(Registry::Full(config)) => Image::pull(image, Some(config)),
                        None => Image::pull(image, None),
                    }
                } else {
                    Image::Use(image)
                };
                PlatformOptions::Container {
                    docker_url: docker_url.as_deref(),
                    image,
                }
            }

            RunsOn::Build {
                name,
                tag,
                dockerfile,
                docker_url,
            } => PlatformOptions::Container {
                image: Image::build(name, dockerfile, tag),
                docker_url: docker_url.as_deref(),
            },

            RunsOn::SshFromGlobalConfig { ssh_config } => {
                let config = self.config.ssh(ssh_config)?;
                let port = config.port.parse::<u16>()?;
                let auth = match &config.userauth {
                    SshUserAuth::Agent => SshAuthOptions::Agent,
                    SshUserAuth::Password { password } => SshAuthOptions::Password { password },
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => SshAuthOptions::Keys {
                        public_key: public_key.as_deref(),
                        private_key,
                    },
                };
                PlatformOptions::Ssh(SshConnectOptions::new(
                    &config.host,
                    port,
                    &config.user,
                    auth,
                ))
            }

            RunsOn::Ssh(config) => {
                let port = config.port.parse::<u16>()?;
                let auth = match &config.userauth {
                    SshUserAuth::Agent => SshAuthOptions::Agent,
                    SshUserAuth::Password { password } => SshAuthOptions::Password { password },
                    SshUserAuth::Keys {
                        public_key,
                        private_key,
                    } => SshAuthOptions::Keys {
                        public_key: public_key.as_deref(),
                        private_key,
                    },
                };
                PlatformOptions::Ssh(SshConnectOptions::new(
                    &config.host,
                    port,
                    &config.user,
                    auth,
                ))
            }
        };

        let conn = self.context.get_conn();
        let platform = PlatformBuilder::default()
            .run_id(&self.run_id)
            .config(self.config.clone())
            .options(options)
            .pipeline_env(&self.pipeline.env)
            .env(self.env.clone())
            .logger(self.logger.clone())
            .conn(conn)
            .build()
            .await?;

        self.context.add_platform(platform.clone()).await?;
        self.platform = Some(platform);
        Ok(())
    }

    async fn dispose_platform(&self) -> Result<()> {
        let Some(platform) = self.platform.as_ref() else {
            bail!("no platform instance for runner");
        };
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
            writeln!(message, "{:<15}: {name}", "Name")?;
        }
        writeln!(message, "{:<15}: {}", "Runs on", &self.pipeline.runs_on)?;
        writeln!(message, "{:<15}: 3", "Version")?;

        self.logger.write_line(message).await
    }

    async fn start(&mut self) -> Result<()> {
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

    fn create_job(&self, name: &str, logger: Arc<Logger>) -> JobRunner {
        JobRunner {
            pipeline: self.pipeline.clone(),
            job_name: name.to_owned(),
            fs: self.fs.clone(),
            run_id: self.run_id.clone(),
            run_start_time: self.run_start_time.clone(),
            config: self.config.clone(),
            logger,
            context: self.context.clone(),
            platform: self.platform.clone(),
            regex_cache: self.regex_cache.clone(),
        }
    }

    async fn prepare_jobs(&self) -> Result<Vec<Option<RunningJob>>> {
        let mut jobs = Vec::new();
        for name in self.pipeline.jobs.keys() {
            self.logger
                .write_line(format!("{:<15}: {}", "Running job", name))
                .await?;
            let logger = Logger::in_memory().into_arc();
            let job = self.create_job(name, logger.clone());
            let handle = spawn(job.run());
            jobs.push(Some(RunningJob::new(name, handle, logger)));
        }
        Ok(jobs)
    }

    async fn run_first_job(&self) -> Result<()> {
        let Some(name) = self.pipeline.jobs.keys().next() else {
            bail!("unable to retrieve job");
        };
        debug!("found only one job so running it in the current context");
        self.create_job(name, self.logger.clone())
            .run()
            .await
            .map(|_| ())
    }

    async fn run_all_jobs(&self) -> Result<()> {
        let mut result = Ok(());
        let mut running_jobs = self.prepare_jobs().await?;

        while running_jobs.iter().any(|x| x.is_some()) {
            for job in running_jobs.iter_mut() {
                let is_finished = job
                    .as_ref()
                    .map(|x| x.handle.is_finished())
                    .unwrap_or_default();

                if is_finished {
                    let Some(running_job) = job.take() else {
                        continue;
                    };

                    let handle_result = running_job.handle.await.map_err(|e| anyhow!(e))?;

                    let message = if handle_result.is_ok() {
                        format!("{:<15}: {}", "Completed job", running_job.name)
                    } else {
                        format!("{:<15}: {}", "Erroneous job", running_job.name)
                    };

                    self.logger.write_line(message).await?;

                    self.logger
                        .write_line(running_job.logger.try_retrieve_output().await?)
                        .await?;

                    result = result.and(handle_result.map(|_| ()));
                }
            }

            sleep(Duration::from_millis(200)).await;
        }

        result.map_err(|_| anyhow!("One or more jobs completed with errors"))
    }

    async fn jobs(&self) -> Result<()> {
        if self.pipeline.jobs.len() == 1 {
            self.run_first_job().await
        } else {
            self.run_all_jobs().await
        }
    }

    async fn execute(mut self) -> Result<()> {
        self.start().await?;

        // using let expression to log the errors and let an empty string be used
        // by the final print_error of main.

        let Err(e) = self.jobs().await else {
            self.stop().await?;
            return Ok(());
        };

        self.logger.write(e.to_string()).await?;
        self.has_faulted = true;
        self.stop().await?;
        bail!("")
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

                if let Ok(message) = signals.try_next() {
                    match message {
                        UnixSignalMessage {
                            signal: UnixSignal::SIGINT,
                            resp_tx,
                        }
                        | UnixSignalMessage {
                            signal: UnixSignal::SIGTERM,
                            resp_tx,
                        }
                        | UnixSignalMessage {
                            signal: UnixSignal::SIGQUIT,
                            resp_tx,
                        } => {
                            runner_handle.abort();

                            logger
                                .write_line(
                                    "Runner interruped. Starting graceful shutdown...".to_owned(),
                                )
                                .await?;

                            context.run_faulted().await?;

                            break resp_tx
                                .send(())
                                .map_err(|_| anyhow!("oneshot response sender dropped"));
                        }
                    }
                }
            }
        })
    }
}
