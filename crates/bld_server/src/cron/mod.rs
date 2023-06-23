use std::{collections::HashMap, str::FromStr, sync::Arc};

use anyhow::{anyhow, bail, Result};
use bld_core::{
    database::{
        cron_job_environment_variables::{
            self, CronJobEnvironmentVariable, InsertCronJobEnvironmentVariable,
        },
        cron_job_variables::{self, CronJobVariable, InsertCronJobVariable},
        cron_jobs::{self, CronJob, InsertCronJob},
        pipeline::{self, Pipeline},
    },
    messages::ExecClientMessage,
    proxies::PipelineFileSystemProxy,
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker};

#[derive(Debug)]
pub struct UpsertJob {
    schedule: String,
    pipeline: String,
    variables: Option<HashMap<String, String>>,
    environment: Option<HashMap<String, String>>,
}

impl UpsertJob {
    pub fn new(
        schedule: String,
        pipeline: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    ) -> Self {
        Self {
            schedule,
            pipeline,
            variables,
            environment,
        }
    }
}

#[derive(Debug)]
pub struct RemoveJob {
    pipeline: String,
}

impl RemoveJob {
    pub fn new(pipeline: String) -> Self {
        Self { pipeline }
    }
}

#[derive(Debug)]
pub struct JobInfo {
    pub schedule: String,
    pub pipeline: String,
    pub variables: Option<HashMap<String, String>>,
    pub environment: Option<HashMap<String, String>>,
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
            let upsert_job = UpsertJob::new(
                job.schedule.to_owned(),
                pipeline.name.to_owned(),
                variables,
                environment,
            );
            let scheduled_job = self.create_scheduled_job(&job_id, &upsert_job)?;
            self.scheduler.add(scheduled_job).await?;
        }

        Ok(())
    }

    fn create_database_job(
        &self,
        conn: &mut SqliteConnection,
        job_id: &Uuid,
        upsert_job: &UpsertJob,
        pipeline_id: &str,
    ) -> Result<CronJob> {
        let job_id_string = job_id.to_string();
        let job = InsertCronJob {
            id: &job_id_string,
            pipeline_id,
            schedule: &upsert_job.schedule,
        };

        let vars: Option<Vec<InsertCronJobVariable>> = upsert_job.variables.as_ref().map(|vars| {
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
            upsert_job.environment.as_ref().map(|envs| {
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
        upsert_job: &UpsertJob,
        pipeline_id: &str,
    ) -> Result<CronJob> {
        let job_id_string = job_id.to_string();
        let job = InsertCronJob {
            id: &job_id_string,
            pipeline_id,
            schedule: &upsert_job.schedule,
        };

        let vars: Option<Vec<InsertCronJobVariable>> = upsert_job.variables.as_ref().map(|vars| {
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
            upsert_job.environment.as_ref().map(|envs| {
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

    fn create_scheduled_job(&self, job_id: &Uuid, upsert_job: &UpsertJob) -> Result<Job> {
        // Compiler complaints about FnMut if parameters are directly used inside the closure
        // so this is the only workaround that works atm.
        let proxy = self.proxy.clone();
        let pool = self.pool.clone();
        let supervisor = self.supervisor.clone();
        let data = ExecClientMessage::EnqueueRun {
            name: upsert_job.pipeline.to_owned(),
            environment: None,
            variables: None,
        };

        let mut job = Job::new_cron_job_async(upsert_job.schedule.as_str(), move |_uuid, _l| {
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
        upsert_job: &UpsertJob,
        pipeline: &Pipeline,
    ) -> Result<()> {
        let job_id = Uuid::new_v4();
        let scheduled_job = self.create_scheduled_job(&job_id, upsert_job)?;
        let scheduled_job_id = scheduled_job.guid();
        self.scheduler.add(scheduled_job).await?;

        if let Err(e) = self.create_database_job(conn, &scheduled_job_id, upsert_job, &pipeline.id)
        {
            self.scheduler.remove(&scheduled_job_id).await?;
            bail!("{e}");
        }

        Ok(())
    }

    async fn update_inner(
        &self,
        conn: &mut SqliteConnection,
        upsert_job: &UpsertJob,
        job: &CronJob,
        pipeline: &Pipeline,
    ) -> Result<()> {
        let job_id = Uuid::from_str(&job.id)?;
        let job = self.update_database_job(conn, &job_id, upsert_job, &pipeline.id)?;

        self.scheduler.remove(&job_id).await?;
        match self.create_scheduled_job(&job_id, upsert_job) {
            Ok(scheduled_job) => self.scheduler.add(scheduled_job).await.map(|_| ())?,
            Err(e) => {
                cron_jobs::delete_by_cron_job_id(conn, &job.id)?;
                bail!("{e}");
            }
        }

        Ok(())
    }

    pub async fn add(&self, upsert_job: &UpsertJob) -> Result<()> {
        let mut conn = self.pool.get()?;
        let pipeline = pipeline::select_by_name(&mut conn, &upsert_job.pipeline)?;
        let job = cron_jobs::select_by_pipeline(&mut conn, &pipeline.id);

        if job.is_ok() {
            bail!("cron job already exists");
        }

        self.add_inner(&mut conn, upsert_job, &pipeline).await
    }

    pub async fn upsert(&self, upsert_job: &UpsertJob) -> Result<()> {
        let mut conn = self.pool.get()?;
        let pipeline = pipeline::select_by_name(&mut conn, &upsert_job.pipeline)?;
        let job = cron_jobs::select_by_pipeline(&mut conn, &pipeline.id);

        match job {
            Ok(job) => {
                self.update_inner(&mut conn, upsert_job, &job, &pipeline)
                    .await
            }
            Err(_) => self.add_inner(&mut conn, upsert_job, &pipeline).await,
        }
    }

    pub async fn remove(&self, remove_job: &RemoveJob) -> Result<()> {
        let mut conn = self.pool.get()?;
        let pipeline = pipeline::select_by_name(&mut conn, &remove_job.pipeline)?;
        let job = cron_jobs::select_by_pipeline(&mut conn, &pipeline.id)?;
        cron_jobs::delete_by_cron_job_id(&mut conn, &job.id)?;

        let job_id = Uuid::from_str(&job.id)?;
        self.scheduler.remove(&job_id).await?;

        Ok(())
    }

    pub fn get(&self) -> Result<Vec<JobInfo>> {
        let mut conn = self.pool.get()?;

        let cron_jobs = cron_jobs::select_all(&mut conn)?;
        let mut response = Vec::with_capacity(cron_jobs.len());

        for job in cron_jobs {
            let pipeline = pipeline::select_by_id(&mut conn, &job.pipeline_id)?;

            let variables = cron_job_variables::select_by_cron_job_id(&mut conn, &job.id)
                .map(Self::variables_into_hash_map)
                .unwrap_or(None);

            let environment =
                cron_job_environment_variables::select_by_cron_job_id(&mut conn, &job.id)
                    .map(Self::environment_into_hash_map)
                    .unwrap_or(None);

            response.push(JobInfo {
                schedule: job.schedule,
                pipeline: pipeline.name.to_owned(),
                variables,
                environment,
            });
        }

        Ok(response)
    }
}
