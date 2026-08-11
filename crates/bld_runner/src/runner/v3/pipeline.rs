use std::{collections::HashMap, fmt::Write, sync::Arc, time::Duration};

use actix_web::rt::spawn;
use anyhow::{Result, anyhow, bail};
use bld_config::BldConfig;
use bld_core::{
    artifacts::Artifacts,
    context::Context,
    fs::FileSystem,
    logger::Logger,
    regex::RegexCache,
    signals::{UnixSignal, UnixSignalMessage, UnixSignalsBackend},
};
use bld_models::dtos::WorkerMessages;
use bld_pkg::PackageManager;
use bld_utils::sync::IntoArc;
use regex::Regex;
use tokio::{sync::mpsc::Sender, time::sleep};
use tracing::debug;

use crate::{
    dag::Dag,
    expr::v3::context::CommonReadonlyRuntimeExprContext,
    pipeline::v3::Pipeline,
    runner::v3::{
        job::JobRunnerOptions,
        state::{JobState, RootState},
    },
};

use super::{
    common::RecursiveFuture,
    job::{JobRunner, RunningJob},
};

pub struct PipelineRunner {
    pub config: Arc<BldConfig>,
    pub fs: Arc<FileSystem>,
    pub logger: Arc<Logger>,
    pub run_ctx: Arc<Context>,
    pub regex_cache: Arc<RegexCache>,
    pub expr_regex: Arc<Regex>,
    pub expr_rctx: Arc<CommonReadonlyRuntimeExprContext>,
    pub pipeline: Arc<Pipeline>,
    pub dag: Dag,
    pub signals: Option<UnixSignalsBackend>,
    pub package_manager: Arc<PackageManager>,
    pub artifacts: Arc<Artifacts>,
    pub ipc: Arc<Option<Sender<WorkerMessages>>>,
    pub is_child: bool,
    pub has_faulted: bool,
}

impl PipelineRunner {
    async fn register_start(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting the pipeline as running in the execution context");
            self.run_ctx
                .set_pipeline_as_running(self.expr_rctx.run_id.to_owned())
                .await?;
        }
        Ok(())
    }

    async fn register_completion(&self) -> Result<()> {
        if !self.is_child {
            debug!("setting state of root pipeline");
            if self.has_faulted {
                self.run_ctx
                    .set_pipeline_as_faulted(self.expr_rctx.run_id.to_owned())
                    .await?;
            } else {
                self.run_ctx
                    .set_pipeline_as_finished(self.expr_rctx.run_id.to_owned())
                    .await?;
            }
        }
        Ok(())
    }

    async fn ipc_send_completed(&self) -> Result<()> {
        if !self.is_child
            && let Some(ipc) = Option::as_ref(&self.ipc)
        {
            debug!("sending message to supervisor for a completed run");
            ipc.send(WorkerMessages::Completed).await?;
        }
        Ok(())
    }

    async fn info(&self) -> Result<()> {
        debug!("printing pipeline informantion");

        let mut message = String::new();

        if let Some(name) = &self.pipeline.name {
            writeln!(message, "{:<15}: {name}", "Name")?;
        }
        writeln!(message, "{:<15}: 3", "Version")?;

        self.logger.write_line(message).await
    }

    async fn start(&mut self) -> Result<()> {
        self.register_start().await?;
        self.info().await?;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        debug!("starting cleanup operations for runner");
        self.register_completion().await?;
        self.ipc_send_completed().await?;
        Ok(())
    }

    async fn create_job(
        &self,
        name: &str,
        logger: Arc<Logger>,
        state: JobState,
    ) -> Result<JobRunner<JobState>> {
        let options = JobRunnerOptions {
            job_name: name.to_string(),
            logger: logger.clone(),
            config: self.config.clone(),
            fs: self.fs.clone(),
            run_ctx: self.run_ctx.clone(),
            pipeline: self.pipeline.clone(),
            regex_cache: self.regex_cache.clone(),
            expr_regex: self.expr_regex.clone(),
            expr_rctx: self.expr_rctx.clone(),
            package_manager: self.package_manager.clone(),
            artifacts: self.artifacts.clone(),
            is_child: self.is_child,
            state,
        };
        JobRunner::new(options).await
    }

    fn create_job_state(
        &self,
        name: &str,
        job_outputs: &HashMap<String, HashMap<String, String>>,
    ) -> Result<JobState> {
        let mut state = JobState::new(name);
        let Some(job) = self.pipeline.jobs.get(name) else {
            bail!("job with name {name} not found");
        };
        for step in &job.steps {
            state.add_node(step.id());
        }
        state.set_job_outputs(job_outputs.clone())?;
        Ok(state)
    }

    async fn prepare_jobs(
        &self,
        names: &[String],
        job_outputs: &HashMap<String, HashMap<String, String>>,
    ) -> Result<Vec<Option<RunningJob>>> {
        let mut jobs = Vec::new();
        for name in names {
            self.logger
                .write_line(format!("{:<15}: {}", "Running job", name))
                .await?;
            let logger = Logger::in_memory().into_arc();
            let state = self.create_job_state(name, job_outputs)?;
            let job = self.create_job(name, logger.clone(), state).await?;
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
        let state = self.create_job_state(name, &HashMap::new())?;
        self.create_job(name, self.logger.clone(), state)
            .await?
            .run()
            .await
            .map(|_| ())
    }

    async fn run_layer(
        &self,
        names: &[String],
        job_outputs: &HashMap<String, HashMap<String, String>>,
    ) -> Result<HashMap<String, HashMap<String, String>>> {
        let mut errors: Vec<String> = Vec::new();
        let mut collected: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut running_jobs = self.prepare_jobs(names, job_outputs).await?;

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

                    let message = match &handle_result {
                        Ok(runner) => {
                            collected.insert(running_job.name.clone(), runner.outputs.clone());
                            format!("{:<15}: {}", "Completed job", running_job.name)
                        }
                        Err(e) => {
                            errors.push(format!("[{}] {e}", running_job.name));
                            format!("{:<15}: {} ({e})", "Erroneous job", running_job.name)
                        }
                    };

                    self.logger.write_line(message).await?;

                    self.logger
                        .write_line(running_job.logger.try_retrieve_output().await?)
                        .await?;
                }
            }

            sleep(Duration::from_millis(200)).await;
        }

        if errors.is_empty() {
            Ok(collected)
        } else {
            Err(anyhow!(errors.join("\n")))
        }
    }

    async fn run_all_jobs(&self) -> Result<()> {
        let mut job_outputs: HashMap<String, HashMap<String, String>> = HashMap::new();
        for layer in self.dag.layers() {
            let layer_outputs = self.run_layer(&layer, &job_outputs).await?;
            job_outputs.extend(layer_outputs);
        }
        Ok(())
    }

    async fn jobs(&self) -> Result<()> {
        if self.pipeline.jobs.len() == 1 {
            self.run_first_job().await
        } else {
            self.run_all_jobs().await
        }
    }

    async fn execute(mut self) -> Result<HashMap<String, String>> {
        self.start().await?;

        // using let expression to log the errors and let an empty string be used
        // by the final print_error of main.

        let Err(e) = self.jobs().await else {
            self.stop().await?;
            return Ok(HashMap::new());
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
                return self.execute().await;
            }

            let context = self.run_ctx.clone();
            let logger = self.logger.clone();
            let mut signals = signals.unwrap();
            let runner_handle = spawn(self.execute());

            loop {
                sleep(Duration::from_millis(200)).await;

                if runner_handle.is_finished() {
                    break runner_handle.await?;
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
                                .map_err(|_| anyhow!("oneshot response sender dropped"))
                                .map(|_| HashMap::new());
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bld_config::BldConfig;
    use bld_core::{
        artifacts::Artifacts, context::Context, fs::FileSystem, logger::Logger, regex::RegexCache,
    };
    use bld_pkg::PackageManager;
    use bld_utils::sync::IntoArc;
    use regex::Regex;

    use crate::{
        dag::Dag,
        expr::v3::{context::CommonReadonlyRuntimeExprContext, parser::EXPR_REGEX},
        job::v3::{Job, Needs},
        outputs::v3::Output,
        pipeline::v3::Pipeline,
    };

    use super::PipelineRunner;

    fn create_runner(jobs: Vec<(&str, Job)>, logger: Arc<Logger>) -> PipelineRunner {
        let config = BldConfig::default().into_arc();
        let mut pipeline = Pipeline::default();
        for (name, job) in jobs {
            pipeline.jobs.insert(name.to_string(), job);
        }

        PipelineRunner {
            fs: FileSystem::local(config.clone()).into_arc(),
            logger,
            run_ctx: Context::mock().into_arc(),
            regex_cache: RegexCache::mock().into_arc(),
            expr_regex: Regex::new(EXPR_REGEX).unwrap().into_arc(),
            expr_rctx: CommonReadonlyRuntimeExprContext::default().into_arc(),
            pipeline: pipeline.into_arc(),
            dag: Dag::default(),
            signals: None,
            package_manager: PackageManager::new(config.clone()).into_arc(),
            artifacts: Artifacts::mock().into_arc(),
            ipc: None.into_arc(),
            is_child: true,
            has_faulted: false,
            config,
        }
    }

    /// An invalid comparison makes the condition evaluation fail, so the job
    /// errors out without needing to run any real steps.
    fn failing_job(condition: &str) -> Job {
        Job {
            condition: Some(condition.to_string()),
            ..Default::default()
        }
    }

    #[actix_web::test]
    async fn run_layer_collects_error_message_of_failing_job() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("producer", failing_job("${{ true == \"James\" }}")),
                ("consumer", Job::default()),
            ],
            logger.clone(),
        );

        let names = vec!["producer".to_string(), "consumer".to_string()];
        let result = runner
            .run_layer(&names, &std::collections::HashMap::new())
            .await;

        let error = result.expect_err("expected the failing job to produce an error");
        let message = error.to_string();

        assert!(
            message.contains("producer"),
            "expected the error to contain the name of the failing job, got: {message}"
        );
        assert!(
            message.contains("cannot compare boolean and text"),
            "expected the error to contain the message of the job, got: {message}"
        );
        assert!(
            !message.contains("consumer"),
            "expected the job that completed to be absent from the error, got: {message}"
        );

        let output = logger.try_retrieve_output().await.unwrap();
        assert!(
            output.contains("Erroneous job") && output.contains("producer"),
            "expected the logger output to name the erroneous job, got: {output}"
        );
        assert!(
            output.contains("cannot compare boolean and text"),
            "expected the logger output to contain the message of the job, got: {output}"
        );
    }

    #[actix_web::test]
    async fn run_layer_collects_error_message_of_every_failing_job() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("producer", failing_job("${{ true == \"James\" }}")),
                ("consumer", failing_job("${{ 1 }} ${{ 2 }}")),
            ],
            logger.clone(),
        );

        let names = vec!["producer".to_string(), "consumer".to_string()];
        let result = runner
            .run_layer(&names, &std::collections::HashMap::new())
            .await;

        let error = result.expect_err("expected the failing jobs to produce an error");
        let message = error.to_string();

        assert!(
            message.contains("[producer] cannot compare boolean and text"),
            "expected the error of the producer job, got: {message}"
        );
        assert!(
            message.contains("[consumer] more than one condition found for step"),
            "expected the error of the consumer job, got: {message}"
        );
    }

    #[actix_web::test]
    async fn run_layer_of_completed_jobs_returns_ok() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![("producer", Job::default()), ("consumer", Job::default())],
            logger.clone(),
        );

        let names = vec!["producer".to_string(), "consumer".to_string()];

        assert!(
            runner
                .run_layer(&names, &std::collections::HashMap::new())
                .await
                .is_ok()
        );
    }

    fn job_with_outputs(needs: Option<Needs>, outputs: Vec<(&str, &str)>) -> Job {
        Job {
            needs,
            outputs: outputs
                .into_iter()
                .map(|(name, value)| (name.to_string(), Output::Simple(value.to_string())))
                .collect(),
            ..Default::default()
        }
    }

    /// Three layers: build (layer 1), publish (layer 2, needs build) and deploy (layer 3,
    /// needs both build and publish). Deploy reads a value straight from build even though
    /// it is two layers away, because build is listed directly in its own needs.
    #[actix_web::test]
    async fn three_layers_collect_and_forward_job_outputs() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("build", job_with_outputs(None, vec![("version", "1.2.3")])),
                (
                    "publish",
                    job_with_outputs(
                        Some(Needs::Single("build".to_string())),
                        vec![("got", "${{ jobs.build.outputs.version }}")],
                    ),
                ),
                (
                    "deploy",
                    job_with_outputs(
                        Some(Needs::Multiple(
                            ["build", "publish"].iter().map(|x| x.to_string()).collect(),
                        )),
                        vec![("final", "${{ jobs.build.outputs.version }}")],
                    ),
                ),
            ],
            logger.clone(),
        );

        let mut job_outputs = std::collections::HashMap::new();

        let layer1 = runner
            .run_layer(&["build".to_string()], &job_outputs)
            .await
            .unwrap();
        job_outputs.extend(layer1);
        assert_eq!(
            job_outputs.get("build").and_then(|m| m.get("version")),
            Some(&"1.2.3".to_string())
        );

        let layer2 = runner
            .run_layer(&["publish".to_string()], &job_outputs)
            .await
            .unwrap();
        job_outputs.extend(layer2);
        assert_eq!(
            job_outputs.get("publish").and_then(|m| m.get("got")),
            Some(&"1.2.3".to_string())
        );

        let layer3 = runner
            .run_layer(&["deploy".to_string()], &job_outputs)
            .await
            .unwrap();
        job_outputs.extend(layer3);
        assert_eq!(
            job_outputs.get("deploy").and_then(|m| m.get("final")),
            Some(&"1.2.3".to_string())
        );
    }

    /// A job that is skipped because its condition fails still shows up in the outputs map
    /// as an empty entry, so a job in the next layer that reads one of its values gets a
    /// clear error naming the job and the missing output, instead of an order violation.
    #[actix_web::test]
    async fn job_skipped_by_condition_gives_empty_outputs_to_the_next_layer() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("build", failing_job("${{ false }}")),
                (
                    "publish",
                    job_with_outputs(
                        Some(Needs::Single("build".to_string())),
                        vec![("got", "${{ jobs.build.outputs.version }}")],
                    ),
                ),
            ],
            logger.clone(),
        );

        // The condition of "build" doesn't fail the run, it just evaluates to false, so the
        // job is skipped rather than erroring.
        let layer1 = runner
            .run_layer(&["build".to_string()], &std::collections::HashMap::new())
            .await
            .unwrap();
        assert_eq!(layer1.get("build"), Some(&std::collections::HashMap::new()));

        let result = runner.run_layer(&["publish".to_string()], &layer1).await;
        let error = result
            .expect_err("expected an error reading the output of a skipped job")
            .to_string();
        assert!(
            error.contains("version") && error.contains("build"),
            "{error}"
        );
    }
}
