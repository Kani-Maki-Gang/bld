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
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker};

#[derive(Debug)]
pub struct JobInfo {
    pub id: String,
    pub schedule: String,
    pub pipeline: String,
    pub variables: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
    pub is_default: bool,
    pub date_created: String,
    pub date_updated: Option<String>,
}

pub struct CronScheduler {
    proxy: Arc<PipelineFileSystemProxy>,
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    supervisor: Arc<SupervisorMessageSender>,
    scheduler: JobScheduler,
}

impl CronScheduler {
    pub async fn new(
        proxy: Arc<PipelineFileSystemProxy>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        supervisor: Arc<SupervisorMessageSender>,
    ) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        scheduler.start().await?;
        let instance = Self {
            proxy,
            pool,
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
        let mut conn = self.pool.get()?;
        let jobs = cron_jobs::select_all(&mut conn)?;

        for job in jobs {
            let pipeline = pipeline::select_by_id(&mut conn, &job.pipeline_id)?;

            let variables = cron_job_variables::select_by_cron_job_id(&mut conn, &job.id)
                .map(Self::variables_into_hash_map)
                .unwrap_or(None);

            let environment =
                cron_job_environment_variables::select_by_cron_job_id(&mut conn, &job.id)
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

    fn create_database_job(
        &self,
        conn: &mut SqliteConnection,
        job_id: &Uuid,
        add_job: &AddJobRequest,
        pipeline_id: &str,
    ) -> Result<CronJob> {
        let job_id_string = job_id.to_string();
        let job = InsertCronJob {
            id: &job_id_string,
            pipeline_id,
            schedule: &add_job.schedule,
            is_default: add_job.is_default,
        };

        let vars: Option<Vec<InsertCronJobVariable>> = add_job.variables.as_ref().map(|vars| {
            vars.iter()
                .map(|(k, v)| InsertCronJobVariable {
                    id: Uuid::new_v4().to_string(),
                    name: k,
                    value: v,
                    cron_job_id: &job_id_string,
                })
                .collect()
        });

        let env: Option<Vec<InsertCronJobEnvironmentVariable>> =
            add_job.environment.as_ref().map(|envs| {
                envs.iter()
                    .map(|(k, v)| InsertCronJobEnvironmentVariable {
                        id: Uuid::new_v4().to_string(),
                        name: k,
                        value: v,
                        cron_job_id: &job_id_string,
                    })
                    .collect()
            });

        cron_jobs::insert(conn, &job, &vars, &env)
    }

    fn update_database_job(
        &self,
        conn: &mut SqliteConnection,
        job_id: &Uuid,
        update_job: &UpdateJobRequest,
        pipeline_id: &str,
    ) -> Result<CronJob> {
        let job_id_string = job_id.to_string();
        let job = UpdateCronJob {
            id: &job_id_string,
            pipeline_id,
            schedule: &update_job.schedule,
        };

        let vars: Option<Vec<InsertCronJobVariable>> = update_job.variables.as_ref().map(|vars| {
            vars.iter()
                .map(|(k, v)| InsertCronJobVariable {
                    id: Uuid::new_v4().to_string(),
                    name: k,
                    value: v,
                    cron_job_id: &job_id_string,
                })
                .collect()
        });

        let env: Option<Vec<InsertCronJobEnvironmentVariable>> =
            update_job.environment.as_ref().map(|envs| {
                envs.iter()
                    .map(|(k, v)| InsertCronJobEnvironmentVariable {
                        id: Uuid::new_v4().to_string(),
                        name: k,
                        value: v,
                        cron_job_id: &job_id_string,
                    })
                    .collect()
            });

        cron_jobs::update(conn, &job, &vars, &env)
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
        let pool = self.pool.clone();
        let supervisor = self.supervisor.clone();
        let data = ExecClientMessage::EnqueueRun {
            name: pipeline.to_owned(),
            environment,
            variables,
        };

        let mut job = Job::new_cron_job_async(schedule, move |_uuid, _l| {
            let proxy = proxy.clone();
            let pool = pool.clone();
            let supervisor = supervisor.clone();
            let data = data.clone();
            Box::pin(async move {
                let _ = enqueue_worker("Cron", proxy, pool, supervisor, data).await;
            })
        })
        .map_err(|e| anyhow!(e))?;

        let mut job_data = job.job_data()?;
        job_data.id.replace(job_id.into());

        Ok(job)
    }

    async fn add_inner(
        &self,
        conn: &mut SqliteConnection,
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

        if let Err(e) = self.create_database_job(conn, &scheduled_job_id, add_job, &pipeline.id) {
            self.scheduler.remove(&scheduled_job_id).await?;
            bail!("{e}");
        }

        Ok(())
    }

    async fn update_inner(
        &self,
        conn: &mut SqliteConnection,
        update_job: &UpdateJobRequest,
        job: &CronJob,
        pipeline: &Pipeline,
    ) -> Result<()> {
        let job_id = Uuid::from_str(&job.id)?;
        self.scheduler.remove(&job_id).await?;

        let variables = update_job.variables.as_ref().cloned();
        let environment = update_job.environment.as_ref().cloned();

        let create_scheduled_job_result = self
            .create_scheduled_job(
                &job_id,
                &update_job.schedule,
                &pipeline.name,
                variables,
                environment,
            )
            .and_then(|scheduled_job| {
                self.update_database_job(conn, &job_id, update_job, &pipeline.id)
                    .map(|_| scheduled_job)
            });

        match create_scheduled_job_result {
            Ok(scheduled_job) => self.scheduler.add(scheduled_job).await.map(|_| ())?,
            Err(e) => bail!("{e}"),
        }

        Ok(())
    }

    pub async fn add(&self, add_job: &AddJobRequest) -> Result<()> {
        let mut conn = self.pool.get()?;
        let pipeline = pipeline::select_by_name(&mut conn, &add_job.pipeline)?;
        let job_exists = add_job.is_default
            && cron_jobs::select_default_by_pipeline(&mut conn, &pipeline.id).is_ok();

        if job_exists {
            bail!("cron job already exists");
        }

        self.add_inner(&mut conn, add_job, &pipeline).await
    }

    pub async fn update(&self, update_job: &UpdateJobRequest) -> Result<()> {
        let mut conn = self.pool.get()?;
        let job = cron_jobs::select_by_id(&mut conn, &update_job.id)?;
        let pipeline = pipeline::select_by_id(&mut conn, &job.pipeline_id)?;
        self.update_inner(&mut conn, update_job, &job, &pipeline)
            .await
    }

    pub async fn upsert_default(&self, schedule: &str, pipeline: &str) -> Result<()> {
        let job = {
            let mut conn = self.pool.get()?;
            let pipeline = pipeline::select_by_name(&mut conn, pipeline)?;
            cron_jobs::select_default_by_pipeline(&mut conn, &pipeline.id)
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
        let mut conn = self.pool.get()?;
        cron_jobs::delete_by_cron_job_id(&mut conn, job_id)?;

        let job_id = Uuid::from_str(job_id)?;
        self.scheduler.remove(&job_id).await?;

        Ok(())
    }

    pub async fn remove_by_pipeline(&self, pipeline: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let jobs = cron_jobs::select_by_pipeline(&mut conn, pipeline)?;

        for job in jobs {
            let job_id = Uuid::from_str(&job.id)?;
            self.scheduler.remove(&job_id).await?;
        }

        cron_jobs::delete_by_pipeline(&mut conn, pipeline)?;

        Ok(())
    }

    fn get_inner(&self, conn: &mut SqliteConnection, jobs: Vec<CronJob>) -> Result<Vec<JobInfo>> {
        let mut response = Vec::with_capacity(jobs.len());

        for job in jobs {
            let pipeline = pipeline::select_by_id(conn, &job.pipeline_id)?;

            let variables = cron_job_variables::select_by_cron_job_id(conn, &job.id)
                .map(Self::variables_into_hash_map)
                .unwrap_or(None);

            let environment = cron_job_environment_variables::select_by_cron_job_id(conn, &job.id)
                .map(Self::environment_into_hash_map)
                .unwrap_or(None);

            response.push(JobInfo {
                id: job.id,
                schedule: job.schedule,
                pipeline: pipeline.name.to_owned(),
                variables,
                environment,
                is_default: job.is_default,
                date_created: job.date_created,
                date_updated: job.date_updated,
            });
        }

        Ok(response)
    }

    pub fn get(&self, filters: &JobFiltersParams) -> Result<Vec<JobInfo>> {
        let mut conn = self.pool.get()?;

        let cron_jobs = cron_jobs::select_with_filters(
            &mut conn,
            filters.id.as_deref(),
            filters.pipeline.as_deref(),
            filters.schedule.as_deref(),
            filters.is_default,
            filters.limit,
        )?;
        self.get_inner(&mut conn, cron_jobs)
    }
}
