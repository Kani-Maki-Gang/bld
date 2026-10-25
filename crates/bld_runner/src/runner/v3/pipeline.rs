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
    dag::{Dag, DagNode},
    expr::v3::{
        context::{CommonReadonlyRuntimeExprContext, START_OF_RUN_WCTX},
        exec::{CommonExprExecutor, eval_all_expressions},
    },
    pipeline::v3::Pipeline,
    runner::v3::{
        job::JobRunnerOptions,
        state::{JobState, RootState, State},
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
            // The name is printed before any job has run, so it can only use the start of
            // run expressions.
            let exec = CommonExprExecutor::new(
                self.pipeline.as_ref(),
                self.expr_rctx.as_ref(),
                &START_OF_RUN_WCTX,
            );
            let name = eval_all_expressions(&exec, &self.expr_regex, name)?;
            writeln!(message, "{:<15}: {name}", "Name")?;
        }
        writeln!(message, "{:<15}: 3", "Version")?;

        self.logger.write_line(message).await
    }

    async fn start(&mut self) -> Result<()> {
        self.register_start().await?;
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

    async fn start_job(
        &self,
        name: &str,
        job_outputs: &HashMap<String, HashMap<String, String>>,
    ) -> Result<RunningJob> {
        let logger = Logger::in_memory().into_arc();
        let state = self.create_job_state(name, job_outputs)?;
        let job = self.create_job(name, logger.clone(), state).await?;
        let handle = spawn(job.run());
        Ok(RunningJob::new(name, handle, logger))
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

    async fn start_ready_jobs(
        &self,
        pending: &mut Vec<&DagNode>,
        running: &mut Vec<RunningJob>,
        concluded: &mut HashMap<String, State>,
        outputs: &HashMap<String, HashMap<String, String>>,
        errors: &mut Vec<String>,
    ) -> Result<()> {
        while let Some(idx) = pending
            .iter()
            .position(|node| node.needs.iter().all(|x| concluded.contains_key(x)))
        {
            let node = pending.remove(idx);
            let blocking_need = node
                .needs
                .iter()
                .find(|x| !concluded.get(*x).is_some_and(State::unblocks_dependents));

            if let Some(need) = blocking_need {
                self.logger
                    .write_line(format!(
                        "{:<15}: {} (job '{need}' did not complete)",
                        "Skipped job", node.name
                    ))
                    .await?;
                concluded.insert(
                    node.name.clone(),
                    State::SkippedByNeed { need: need.clone() },
                );
                continue;
            }

            let job_outputs = node
                .needs
                .iter()
                .filter_map(|x| outputs.get(x).map(|y| (x.clone(), y.clone())))
                .collect();

            self.logger
                .write_line(format!("{:<15}: {}", "Running job", node.name))
                .await?;

            // A job that fails to start is concluded as failed, the same as a job that fails
            // while running, so that the jobs that are already running are not abandoned.
            match self.start_job(&node.name, &job_outputs).await {
                Ok(job) => running.push(job),
                Err(e) => {
                    concluded.insert(
                        node.name.clone(),
                        State::Failed {
                            error: e.to_string(),
                        },
                    );
                    errors.push(format!("[{}] {e}", node.name));
                    self.logger
                        .write_line(format!("{:<15}: {} ({e})", "Erroneous job", node.name))
                        .await?;
                }
            }
        }
        Ok(())
    }

    async fn collect_finished_jobs(
        &self,
        running: &mut Vec<RunningJob>,
        concluded: &mut HashMap<String, State>,
        outputs: &mut HashMap<String, HashMap<String, String>>,
        errors: &mut Vec<String>,
    ) -> Result<()> {
        while let Some(idx) = running.iter().position(|x| x.handle.is_finished()) {
            let running_job = running.remove(idx);
            let handle_result = running_job.handle.await.map_err(|e| anyhow!(e))?;

            let message = match &handle_result {
                Ok(runner) => {
                    let state = runner.options.state.get_state().clone();
                    if state.unblocks_dependents() {
                        outputs.insert(running_job.name.clone(), runner.outputs.clone());
                    }
                    concluded.insert(running_job.name.clone(), state);
                    if runner.is_skipped() {
                        format!("{:<15}: {}", "Skipped job", running_job.name)
                    } else {
                        format!("{:<15}: {}", "Completed job", running_job.name)
                    }
                }
                Err(e) => {
                    concluded.insert(
                        running_job.name.clone(),
                        State::Failed {
                            error: e.to_string(),
                        },
                    );
                    errors.push(format!("[{}] {e}", running_job.name));
                    format!("{:<15}: {} ({e})", "Erroneous job", running_job.name)
                }
            };

            self.logger.write_line(message).await?;

            self.logger
                .write_line(running_job.logger.try_retrieve_output().await?)
                .await?;
        }
        Ok(())
    }

    async fn run_all_jobs(&self) -> Result<HashMap<String, HashMap<String, String>>> {
        let mut pending: Vec<&DagNode> = self.dag.nodes.iter().collect();
        let mut running: Vec<RunningJob> = Vec::new();
        let mut concluded: HashMap<String, State> = HashMap::new();
        let mut outputs: HashMap<String, HashMap<String, String>> = HashMap::new();
        let mut errors: Vec<String> = Vec::new();

        loop {
            self.start_ready_jobs(
                &mut pending,
                &mut running,
                &mut concluded,
                &outputs,
                &mut errors,
            )
            .await?;

            if running.is_empty() {
                break;
            }

            sleep(Duration::from_millis(200)).await;

            self.collect_finished_jobs(&mut running, &mut concluded, &mut outputs, &mut errors)
                .await?;
        }

        if !pending.is_empty() {
            let names: Vec<&str> = pending.iter().map(|x| x.name.as_str()).collect();
            bail!("unable to schedule jobs: {}", names.join(", "));
        }

        if !errors.is_empty() {
            bail!(errors.join("\n"));
        }

        Ok(outputs)
    }

    async fn jobs(&self) -> Result<()> {
        if self.pipeline.jobs.len() == 1 {
            self.run_first_job().await
        } else {
            self.run_all_jobs().await.map(|_| ())
        }
    }

    async fn execute(mut self) -> Result<HashMap<String, String>> {
        self.start().await?;

        // using let expression to log the errors and let an empty string be used
        // by the final print_error of main.

        // The info is printed here so that a failure evaluating the pipeline name
        // marks the run as faulted, the same as a failure of a job.
        let result = match self.info().await {
            Ok(()) => self.jobs().await,
            Err(e) => Err(e),
        };

        let Err(e) = result else {
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
                            logger
                                .write_line(
                                    "Runner interruped. Starting graceful shutdown...".to_owned(),
                                )
                                .await?;

                            context.cancel();
                            let result = runner_handle.await;

                            context.run_faulted().await?;

                            resp_tx
                                .send(())
                                .map_err(|_| anyhow!("oneshot response sender dropped"))?;

                            break result?;
                        }
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

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
        external::v3::External,
        job::v3::{Job, Needs},
        outputs::v3::Output,
        pipeline::v3::Pipeline,
        runner::v3::test_utils::TempDir,
        step::v3::{ShellCommand, Step},
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
            dag: Dag::try_from(&pipeline).unwrap(),
            pipeline: pipeline.into_arc(),
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
    async fn run_all_jobs_collects_error_message_of_failing_job() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("producer", failing_job("${{ true == \"James\" }}")),
                ("consumer", Job::default()),
            ],
            logger.clone(),
        );

        let result = runner.run_all_jobs().await;

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
    async fn run_all_jobs_collects_error_message_of_every_failing_job() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("producer", failing_job("${{ true == \"James\" }}")),
                ("consumer", failing_job("${{ 1 }} ${{ 2 }}")),
            ],
            logger.clone(),
        );

        let result = runner.run_all_jobs().await;

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
    async fn run_all_jobs_of_completed_jobs_returns_ok() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![("producer", Job::default()), ("consumer", Job::default())],
            logger.clone(),
        );

        assert!(runner.run_all_jobs().await.is_ok());
    }

    fn machine_job(needs: Option<Needs>, steps: Vec<(&str, &str)>) -> Job {
        Job {
            needs,
            steps: steps
                .into_iter()
                .map(|(id, run)| {
                    Step::ComplexSh(Box::new(ShellCommand {
                        id: id.to_string(),
                        run: run.to_string(),
                        ..Default::default()
                    }))
                })
                .collect(),
            ..Default::default()
        }
    }

    fn with_root_dir(mut runner: PipelineRunner, dir: &TempDir) -> PipelineRunner {
        runner.config = BldConfig {
            root_dir: dir.root_dir(),
            ..Default::default()
        }
        .into_arc();
        runner
    }

    #[actix_web::test]
    async fn run_all_jobs_of_parallel_machine_jobs_keeps_each_working_directory() {
        let dir = TempDir::new("parallel_machine_jobs");
        let logger = Logger::in_memory().into_arc();
        let runner = with_root_dir(
            create_runner(
                vec![
                    ("fast", machine_job(None, vec![("s", "echo fast done")])),
                    (
                        "slow",
                        machine_job(
                            None,
                            vec![
                                ("a", "echo data > shared.txt && sleep 1"),
                                ("b", "cat shared.txt"),
                            ],
                        ),
                    ),
                ],
                logger.clone(),
            ),
            &dir,
        );

        let result = runner.run_all_jobs().await;

        let output = logger.try_retrieve_output().await.unwrap();
        assert!(
            result.is_ok(),
            "expected both machine jobs to complete, got: {result:?}, output: {output}"
        );
        assert!(
            output.contains("data"),
            "expected the slow job to read its own file, got: {output}"
        );
    }

    #[actix_web::test]
    async fn action_steps_fall_back_to_the_job_working_directory() {
        let dir = TempDir::new("action_job_working_directory");
        dir.write(
            "pwd.yaml",
            r#"
version: 3
type: action
name: Pwd

steps:
  - id: pwd
    run: pwd
"#,
        );
        let working_dir = format!("{}/job_working_dir", dir.root_dir());
        std::fs::create_dir_all(&working_dir).unwrap();

        let logger = Logger::in_memory().into_arc();
        let job = Job {
            working_dir: Some(working_dir.clone()),
            steps: vec![Step::ExternalFile(Box::new(External {
                id: "pwd".to_string(),
                uses: "pwd.yaml".to_string(),
                ..Default::default()
            }))],
            ..Default::default()
        };
        let mut runner = create_runner(vec![("job", job)], logger.clone());

        runner.config = BldConfig {
            root_dir: dir.root_dir(),
            ..Default::default()
        }
        .into_arc();

        runner.fs = FileSystem::local(runner.config.clone()).into_arc();
        runner.package_manager = PackageManager::new(runner.config.clone()).into_arc();

        let result = runner.run_all_jobs().await;

        let output = logger.try_retrieve_output().await.unwrap();
        assert!(result.is_ok(), "error: {result:?}, output: {output}");
        assert!(output.contains(&working_dir), "{output}");
    }

    #[actix_web::test]
    async fn cancelling_the_run_stops_the_running_jobs() {
        let dir = TempDir::new("cancel_running_jobs");
        let logger = Logger::in_memory().into_arc();
        let mut runner = create_runner(
            vec![
                (
                    "first",
                    machine_job(None, vec![("s", "sleep 1 && echo x > marker")]),
                ),
                (
                    "second",
                    machine_job(None, vec![("s", "sleep 1 && echo x > marker")]),
                ),
            ],
            logger.clone(),
        );
        runner.config = BldConfig {
            root_dir: dir.root_dir(),
            ..Default::default()
        }
        .into_arc();

        let run_ctx = runner.run_ctx.clone();
        actix_web::rt::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            run_ctx.cancel();
        });

        let result = runner.run_all_jobs().await;

        let message = result
            .expect_err("expected the cancelled jobs to error out")
            .to_string();
        assert!(
            message.contains("[first] job was cancelled")
                && message.contains("[second] job was cancelled"),
            "{message}"
        );

        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let markers = walk_files(std::path::Path::new(&dir.root_dir()), "marker");
        assert!(
            markers.is_empty(),
            "expected no marker files, got: {markers:?}"
        );
    }

    fn walk_files(dir: &std::path::Path, name: &str) -> Vec<std::path::PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return vec![];
        };
        entries
            .flatten()
            .flat_map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    walk_files(&path, name)
                } else if entry.file_name() == name {
                    vec![path]
                } else {
                    vec![]
                }
            })
            .collect()
    }

    fn needs(names: &[&str]) -> Option<Needs> {
        match names {
            [single] => Some(Needs::Single(single.to_string())),
            many => Some(Needs::Multiple(
                many.iter().map(|x| x.to_string()).collect(),
            )),
        }
    }

    fn job_needing(names: &[&str]) -> Job {
        Job {
            needs: needs(names),
            ..Default::default()
        }
    }

    #[actix_web::test]
    async fn failing_job_skips_its_dependents_but_not_unrelated_jobs() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("a", failing_job("${{ true == \"James\" }}")),
                ("b", Job::default()),
                ("c", job_needing(&["a"])),
                ("d", job_needing(&["b"])),
                ("e", job_needing(&["c"])),
            ],
            logger.clone(),
        );

        let error = runner
            .run_all_jobs()
            .await
            .expect_err("expected the failing job to fail the run")
            .to_string();
        assert!(error.contains("[a]"), "{error}");
        assert!(!error.contains("[c]") && !error.contains("[e]"), "{error}");

        let output = logger.try_retrieve_output().await.unwrap();
        assert!(output.contains("Completed job  : d"), "{output}");
        assert!(
            output.contains("Skipped job    : c (job 'a' did not complete)"),
            "{output}"
        );
        assert!(
            output.contains("Skipped job    : e (job 'c' did not complete)"),
            "{output}"
        );
        assert!(!output.contains("Running job    : c"), "{output}");
        assert!(!output.contains("Running job    : e"), "{output}");
    }

    #[actix_web::test]
    async fn job_failing_to_start_skips_its_dependents_but_not_unrelated_jobs() {
        let dir = TempDir::new("job_failing_to_start");
        let logger = Logger::in_memory().into_arc();
        let mut broken = job_needing(&["b"]);
        broken
            .env
            .insert("X".to_string(), "${{ true == \"James\" }}".to_string());
        let runner = with_root_dir(
            create_runner(
                vec![
                    ("slow", machine_job(None, vec![("s", "sleep 1")])),
                    ("b", Job::default()),
                    ("c", broken),
                    ("d", job_needing(&["c"])),
                ],
                logger.clone(),
            ),
            &dir,
        );

        let error = runner
            .run_all_jobs()
            .await
            .expect_err("expected the job that failed to start to fail the run")
            .to_string();
        assert!(error.contains("[c]"), "{error}");

        let output = logger.try_retrieve_output().await.unwrap();
        assert!(output.contains("Erroneous job  : c"), "{output}");
        assert!(output.contains("Completed job  : slow"), "{output}");
        assert!(
            output.contains("Skipped job    : d (job 'c' did not complete)"),
            "{output}"
        );
    }

    #[actix_web::test]
    async fn job_starts_without_waiting_for_jobs_it_does_not_need() {
        let dir = TempDir::new("eager_job_scheduling");
        let logger = Logger::in_memory().into_arc();
        let runner = with_root_dir(
            create_runner(
                vec![
                    ("fast", machine_job(None, vec![("s", "echo fast")])),
                    ("slow", machine_job(None, vec![("s", "sleep 2")])),
                    (
                        "after_fast",
                        machine_job(needs(&["fast"]), vec![("s", "echo after fast")]),
                    ),
                ],
                logger.clone(),
            ),
            &dir,
        );

        let result = runner.run_all_jobs().await;
        let output = logger.try_retrieve_output().await.unwrap();
        assert!(result.is_ok(), "{result:?}, output: {output}");

        let after_fast = output
            .find("Completed job  : after_fast")
            .unwrap_or_else(|| panic!("after_fast did not complete: {output}"));
        let slow = output
            .find("Completed job  : slow")
            .unwrap_or_else(|| panic!("slow did not complete: {output}"));
        assert!(
            after_fast < slow,
            "expected after_fast to finish before slow, got: {output}"
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

    /// Deploy reads a value straight from build even though build is two levels away,
    /// because build is listed directly in its own needs.
    #[actix_web::test]
    async fn chained_jobs_collect_and_forward_job_outputs() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("build", job_with_outputs(None, vec![("version", "1.2.3")])),
                (
                    "publish",
                    job_with_outputs(
                        needs(&["build"]),
                        vec![("got", "${{ jobs.build.outputs.version }}")],
                    ),
                ),
                (
                    "deploy",
                    job_with_outputs(
                        needs(&["build", "publish"]),
                        vec![("final", "${{ jobs.build.outputs.version }}")],
                    ),
                ),
            ],
            logger.clone(),
        );

        let job_outputs = runner.run_all_jobs().await.unwrap();

        assert_eq!(
            job_outputs.get("build").and_then(|m| m.get("version")),
            Some(&"1.2.3".to_string())
        );
        assert_eq!(
            job_outputs.get("publish").and_then(|m| m.get("got")),
            Some(&"1.2.3".to_string())
        );
        assert_eq!(
            job_outputs.get("deploy").and_then(|m| m.get("final")),
            Some(&"1.2.3".to_string())
        );
    }

    #[actix_web::test]
    async fn job_skipped_by_condition_skips_its_dependents() {
        let logger = Logger::in_memory().into_arc();
        let runner = create_runner(
            vec![
                ("build", failing_job("${{ false }}")),
                (
                    "publish",
                    job_with_outputs(
                        needs(&["build"]),
                        vec![("got", "${{ jobs.build.outputs.version }}")],
                    ),
                ),
                ("deploy", job_needing(&["publish"])),
            ],
            logger.clone(),
        );

        let result = runner.run_all_jobs().await;

        let output = logger.try_retrieve_output().await.unwrap();
        assert!(result.is_ok(), "{result:?}, output: {output}");
        assert!(output.contains("Skipped job    : build"), "{output}");
        assert!(
            output.contains("Skipped job    : publish (job 'build' did not complete)"),
            "{output}"
        );
        assert!(
            output.contains("Skipped job    : deploy (job 'publish' did not complete)"),
            "{output}"
        );
        assert!(!output.contains("Running job    : publish"), "{output}");
        assert!(!output.contains("Running job    : deploy"), "{output}");
    }

    fn runner_with_name(name: &str, inputs: Vec<(&str, &str)>) -> PipelineRunner {
        let logger = Logger::in_memory().into_arc();
        let mut runner = create_runner(vec![], logger);
        let inputs: HashMap<String, String> = inputs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        runner.expr_rctx = CommonReadonlyRuntimeExprContext {
            inputs: crate::expr::v3::context::text_inputs(&inputs),
            ..Default::default()
        }
        .into_arc();
        runner.pipeline = Pipeline {
            name: Some(name.to_string()),
            ..Default::default()
        }
        .into_arc();
        runner
    }

    #[actix_web::test]
    async fn info_evaluates_pipeline_name_with_input() {
        let runner = runner_with_name("Build ${{ inputs.label }}", vec![("label", "nightly")]);

        runner.info().await.unwrap();

        let output = runner.logger.try_retrieve_output().await.unwrap();
        assert!(output.contains("Build nightly"), "{output}");
        assert!(!output.contains("${{"), "{output}");
    }

    #[actix_web::test]
    async fn info_with_runtime_expression_in_pipeline_name_fails() {
        let runner = runner_with_name("Build ${{ matrix.os }}", vec![]);

        assert!(runner.info().await.is_err());
    }
}
