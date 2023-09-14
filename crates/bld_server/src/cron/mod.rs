use std::{collections::HashMap, str::FromStr, sync::Arc};

use anyhow::{anyhow, bail, Result};
use bld_core::{
    database::{
        cron_job_environment_variables::{
            self, CronJobEnvironmentVariable, InsertCronJobEnvironmentVariable,
        },
        cron_job_variables::{self, CronJobVariable, InsertCronJobVariable},
        cron_jobs::{self, CronJob, InsertCronJob, UpdateCronJob},
        pipeline::{self, Pipeline},
    },
    messages::ExecClientMessage,
    proxies::PipelineFileSystemProxy,
    requests::{AddJobRequest, JobFiltersParams, UpdateJobRequest},
    responses::CronJobResponse,
};
use sea_orm::DatabaseConnection;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker};

pub struct CronScheduler {
    proxy: Arc<PipelineFileSystemProxy>,
    conn: Arc<DatabaseConnection>,
    supervisor: Arc<SupervisorMessageSender>,
    scheduler: JobScheduler,
}

impl CronScheduler {
    pub async fn new(
        proxy: Arc<PipelineFileSystemProxy>,
        conn: Arc<DatabaseConnection>,
        supervisor: Arc<SupervisorMessageSender>,
    ) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        scheduler.start().await?;
        let instance = Self {
            proxy,
            conn,
            supervisor,
            scheduler,
        };
        instance.load_jobs().await?;
        Ok(instance)
    }

    fn variables_into_hash_map(variables: Vec<CronJobVariable>) -> Option<HashMap<String, String>> {
        let variables: HashMap<String, String> =
            variables.into_iter().map(|v| (v.name, v.value)).collect();

        if !variables.is_empty() {
            Some(variables)
        } else {
            None
        }
    }

    fn environment_into_hash_map(
        environment: Vec<CronJobEnvironmentVariable>,
    ) -> Option<HashMap<String, String>> {
        let environment: HashMap<String, String> =
            environment.into_iter().map(|e| (e.name, e.value)).collect();

        if !environment.is_empty() {
            Some(environment)
        } else {
            None
        }
    }

    async fn load_jobs(&self) -> Result<()> {
        let conn = self.conn.as_ref();
        let jobs = cron_jobs::select_all(conn).await?;

        for job in jobs {
            let pipeline = pipeline::select_by_id(conn, &job.pipeline_id).await?;

            let variables = cron_job_variables::select_by_cron_job_id(conn, &job.id)
                .await
                .map(Self::variables_into_hash_map)
                .unwrap_or(None);

            let environment = cron_job_environment_variables::select_by_cron_job_id(conn, &job.id)
                .await
                .map(Self::environment_into_hash_map)
                .unwrap_or(None);

            let job_id = Uuid::from_str(&job.id)?;
            let scheduled_job = self.create_scheduled_job(
                &job_id,
                &job.schedule,
                &pipeline.name,
                variables,
                environment,
            )?;

            self.scheduler.add(scheduled_job).await?;
        }

        Ok(())
    }

    async fn create_database_job(
        &self,
        conn: &DatabaseConnection,
        job_id: &Uuid,
        add_job: &AddJobRequest,
        pipeline_id: &str,
    ) -> Result<()> {
        let job_id_str = job_id.to_string();
        let job = InsertCronJob {
            id: job_id_str.to_owned(),
            pipeline_id: pipeline_id.to_owned(),
            schedule: add_job.schedule.to_owned(),
            is_default: add_job.is_default,
        };

        let vars: Option<Vec<_>> = add_job.variables.as_ref().map(|vars| {
            vars.iter()
                .map(|kv| InsertCronJobVariable::new(kv, &job_id_str))
                .collect()
        });

        let env: Option<Vec<_>> = add_job.environment.as_ref().map(|envs| {
            envs.iter()
                .map(|kv| InsertCronJobEnvironmentVariable::new(kv, &job_id_str))
                .collect()
        });

        cron_jobs::insert(conn, &job, &vars, &env).await
    }

    async fn update_database_job(
        &self,
        conn: &DatabaseConnection,
        job_id: &Uuid,
        update_job: &UpdateJobRequest,
    ) -> Result<CronJob> {
        let job_id_str = job_id.to_string();
        let job = UpdateCronJob {
            id: job_id_str.to_owned(),
            schedule: update_job.schedule.to_owned(),
        };

        let vars: Option<Vec<_>> = update_job.variables.as_ref().map(|vars| {
            vars.iter()
                .map(|kv| InsertCronJobVariable::new(kv, &job_id_str))
                .collect()
        });

        let env: Option<Vec<_>> = update_job.environment.as_ref().map(|envs| {
            envs.iter()
                .map(|kv| InsertCronJobEnvironmentVariable::new(kv, &job_id_str))
                .collect()
        });

        cron_jobs::update(conn, &job, &vars, &env).await?;
        cron_jobs::select_by_id(conn, &job.id).await
    }

    fn create_scheduled_job(
        &self,
        job_id: &Uuid,
        schedule: &str,
        pipeline: &str,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    ) -> Result<Job> {
        // Compiler complaints about FnMut if parameters are directly used inside the closure
        // so this is the only workaround that works atm.
        let proxy = self.proxy.clone();
        let conn = self.conn.clone();
        let supervisor = self.supervisor.clone();
        let data = ExecClientMessage::EnqueueRun {
            name: pipeline.to_owned(),
            environment,
            variables,
        };

        let mut job = Job::new_cron_job_async(schedule, move |_uuid, _l| {
            let proxy = proxy.clone();
            let conn = conn.clone();
            let supervisor = supervisor.clone();
            let data = data.clone();
            Box::pin(async move {
                let _ = enqueue_worker("Cron", proxy, conn, supervisor, data).await;
            })
        })
        .map_err(|e| anyhow!(e))?;

        let mut job_data = job.job_data()?;
        job_data.id.replace(job_id.into());

        Ok(job)
    }

    async fn add_inner(
        &self,
        conn: &DatabaseConnection,
        add_job: &AddJobRequest,
        pipeline: &Pipeline,
    ) -> Result<()> {
        let job_id = Uuid::new_v4();

        let variables = add_job.variables.as_ref().cloned();
        let environment = add_job.environment.as_ref().cloned();

        let scheduled_job = self.create_scheduled_job(
            &job_id,
            &add_job.schedule,
            &pipeline.name,
            variables,
            environment,
        )?;
        let scheduled_job_id = scheduled_job.guid();
        self.scheduler.add(scheduled_job).await?;

        if let Err(e) = self
            .create_database_job(conn, &scheduled_job_id, add_job, &pipeline.id)
            .await
        {
            self.scheduler.remove(&scheduled_job_id).await?;
            bail!("{e}");
        }

        Ok(())
    }

    async fn update_inner(
        &self,
        conn: &DatabaseConnection,
        update_job: &UpdateJobRequest,
        job: &CronJob,
        pipeline: &Pipeline,
    ) -> Result<()> {
        let job_id = Uuid::from_str(&job.id)?;
        self.scheduler.remove(&job_id).await?;

        let variables = update_job.variables.as_ref().cloned();
        let environment = update_job.environment.as_ref().cloned();

        let scheduled_job = self.create_scheduled_job(
            &job_id,
            &update_job.schedule,
            &pipeline.name,
            variables,
            environment,
        )?;

        self.update_database_job(conn, &job_id, update_job).await?;
        self.scheduler.add(scheduled_job).await?;

        Ok(())
    }

    pub async fn add(&self, add_job: &AddJobRequest) -> Result<()> {
        let conn = self.conn.as_ref();
        let pipeline = pipeline::select_by_name(conn, &add_job.pipeline).await?;
        let job_exists = add_job.is_default
            && cron_jobs::select_default_by_pipeline(conn, &pipeline.id)
                .await
                .is_ok();

        if job_exists {
            bail!("cron job already exists");
        }

        self.add_inner(conn, add_job, &pipeline).await
    }

    pub async fn update(&self, update_job: &UpdateJobRequest) -> Result<()> {
        let conn = self.conn.as_ref();
        let job = cron_jobs::select_by_id(conn, &update_job.id).await?;
        let pipeline = pipeline::select_by_id(conn, &job.pipeline_id).await?;
        self.update_inner(conn, update_job, &job, &pipeline).await
    }

    pub async fn upsert_default(&self, schedule: &str, pipeline: &str) -> Result<()> {
        let job = {
            let conn = self.conn.as_ref();
            let pipeline = pipeline::select_by_name(conn, pipeline).await?;
            cron_jobs::select_default_by_pipeline(conn, &pipeline.id).await
        };
        match job {
            Ok(job) => {
                let update_job = UpdateJobRequest::new(job.id, schedule.to_owned(), None, None);
                self.update(&update_job).await
            }
            Err(_) => {
                let add_job =
                    AddJobRequest::new(schedule.to_owned(), pipeline.to_owned(), None, None, true);
                self.add(&add_job).await
            }
        }
    }

    pub async fn remove(&self, job_id: &str) -> Result<()> {
        let conn = self.conn.as_ref();
        cron_jobs::select_by_id(conn, job_id).await?;
        let scheduled_job_id = Uuid::from_str(job_id)?;
        self.scheduler.remove(&scheduled_job_id).await?;
        cron_jobs::delete_by_cron_job_id(conn, job_id).await?;
        Ok(())
    }

    pub async fn remove_scheduled_jobs(&self, pipeline: &str) -> Result<()> {
        let jobs = cron_jobs::select_by_pipeline(self.conn.as_ref(), pipeline).await?;

        for job in jobs {
            let job_id = Uuid::from_str(&job.id)?;
            self.scheduler.remove(&job_id).await?;
        }

        Ok(())
    }

    pub async fn remove_by_pipeline(&self, pipeline: &str) -> Result<()> {
        let conn = self.conn.as_ref();
        let jobs = cron_jobs::select_by_pipeline(conn, pipeline).await?;

        for job in jobs {
            let job_id = Uuid::from_str(&job.id)?;
            self.scheduler.remove(&job_id).await?;
        }

        let pipeline = pipeline::select_by_name(conn, pipeline).await?;
        cron_jobs::delete_by_pipeline(conn, &pipeline.id).await?;

        Ok(())
    }

    async fn get_inner(
        &self,
        conn: &DatabaseConnection,
        jobs: Vec<CronJob>,
    ) -> Result<Vec<CronJobResponse>> {
        let mut response = Vec::with_capacity(jobs.len());

        for job in jobs {
            let pipeline = pipeline::select_by_id(conn, &job.pipeline_id).await?;

            let variables = cron_job_variables::select_by_cron_job_id(conn, &job.id)
                .await
                .map(Self::variables_into_hash_map)
                .unwrap_or(None);

            let environment = cron_job_environment_variables::select_by_cron_job_id(conn, &job.id)
                .await
                .map(Self::environment_into_hash_map)
                .unwrap_or(None);

            response.push(CronJobResponse {
                id: job.id,
                schedule: job.schedule,
                pipeline: pipeline.name.to_owned(),
                variables,
                environment,
                is_default: job.is_default,
                date_created: job.date_created.to_string(),
                date_updated: Some(job.date_updated.to_string()),
            });
        }

        Ok(response)
    }

    pub async fn get(&self, filters: &JobFiltersParams) -> Result<Vec<CronJobResponse>> {
        let conn = self.conn.as_ref();

        let cron_jobs = cron_jobs::select_with_filters(
            conn,
            filters.id.as_deref(),
            filters.pipeline.as_deref(),
            filters.schedule.as_deref(),
            filters.is_default,
            filters.limit,
        )
        .await?;
        self.get_inner(conn, cron_jobs).await
    }
}
