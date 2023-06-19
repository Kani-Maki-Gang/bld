use std::{collections::HashMap, str::FromStr, sync::Arc};

use actix::spawn;
use anyhow::{anyhow, Result};
use bld_core::{
    database::{
        cron_job_environment_variables::InsertCronJobEnvironmentVariable,
        cron_job_variables::InsertCronJobVariable,
        cron_jobs::{self, InsertCronJob},
        pipeline,
    },
    messages::ExecClientMessage,
    proxies::PipelineFileSystemProxy,
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::error;
use uuid::Uuid;

use crate::supervisor::{channel::SupervisorMessageSender, helpers::enqueue_worker};

#[derive(Debug)]
enum CronSchedulerMessage {
    Add {
        schedule: String,
        pipeline: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    },
    Remove {
        pipeline: String,
    },
}

struct CronSchedulerReceiver {
    proxy: Arc<PipelineFileSystemProxy>,
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    supervisor: Arc<SupervisorMessageSender>,
    rx: Receiver<CronSchedulerMessage>,
    scheduler: JobScheduler,
    map: Arc<Mutex<HashMap<Uuid, Uuid>>>,
}

impl CronSchedulerReceiver {
    pub async fn new(
        proxy: Arc<PipelineFileSystemProxy>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        supervisor: Arc<SupervisorMessageSender>,
        rx: Receiver<CronSchedulerMessage>,
    ) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        Ok(Self {
            proxy,
            pool,
            supervisor,
            rx,
            scheduler,
            map: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn receive(mut self) -> Result<()> {
        self.scheduler.start().await?;

        while let Some(message) = self.rx.recv().await {
            let result = match message {
                CronSchedulerMessage::Add {
                    schedule,
                    pipeline,
                    variables,
                    environment,
                } => self.add(schedule, pipeline, variables, environment).await,

                CronSchedulerMessage::Remove { pipeline } => self.remove(&pipeline).await,
            };
            if let Err(e) = result {
                error!("{e}");
            }
        }
        Ok(())
    }

    async fn add(
        &self,
        schedule: String,
        pipeline: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let mut conn = self.pool.get()?;
        let pipeline = pipeline::select_by_name(&mut conn, &pipeline)?;
        let job_id = Uuid::new_v4();
        let job_id_string = job_id.to_string();
        let job = InsertCronJob {
            id: &job_id_string,
            pipeline_id: &pipeline.id,
            schedule: &schedule,
        };

        let vars: Option<Vec<InsertCronJobVariable>> = variables.as_ref().map(|vars| {
            vars.iter()
                .map(|(k, v)| InsertCronJobVariable {
                    id: Uuid::new_v4().to_string(),
                    name: k,
                    value: v,
                    cron_job_id: &job_id_string,
                })
                .collect()
        });

        let env: Option<Vec<InsertCronJobEnvironmentVariable>> = environment.as_ref().map(|envs| {
            envs.iter()
                .map(|(k, v)| InsertCronJobEnvironmentVariable {
                    id: Uuid::new_v4().to_string(),
                    name: k,
                    value: v,
                    cron_job_id: &job_id_string,
                })
                .collect()
        });

        let _job = cron_jobs::insert(&mut conn, &job, &vars, &env)?;

        // Compiler complaints about FnMut if parameters are directly used inside the closure
        // so this is the only workaround that works atm.
        let proxy = self.proxy.clone();
        let pool = self.pool.clone();
        let supervisor = self.supervisor.clone();
        let data = ExecClientMessage::EnqueueRun {
            name: pipeline.name.clone(),
            environment,
            variables,
        };
        let scheduled_job = Job::new_cron_job(schedule.as_str(), move |_uuid, _l| {
            let proxy = proxy.clone();
            let pool = pool.clone();
            let supervisor = supervisor.clone();
            let data = data.clone();
            let _ = enqueue_worker("Cron", proxy, pool, supervisor, data)
                .map(|_| ())
                .map_err(|e| error!("{e}"));
        })?;

        let mut map = self.map.lock().await;
        map.insert(job_id, scheduled_job.guid());

        self.scheduler.add(scheduled_job).await?;

        Ok(())
    }

    async fn remove(&self, pipeline: &str) -> Result<()> {
        let mut conn = self.pool.get()?;
        let pipeline = pipeline::select_by_name(&mut conn, pipeline)?;
        let job = cron_jobs::select_by_pipeline(&mut conn, &pipeline.id)?;
        cron_jobs::delete_by_cron_job_id(&mut conn, &job.id)?;

        let map = self.map.lock().await;
        let job_id = Uuid::from_str(&job.id)?;
        if let Some(scheduled_job_id) = map.get(&job_id) {
            self.scheduler.remove(scheduled_job_id).await?;
        }

        Ok(())
    }
}

pub struct CronScheduler {
    tx: Sender<CronSchedulerMessage>,
}

impl CronScheduler {
    pub fn new(
        proxy: Arc<PipelineFileSystemProxy>,
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        supervisor: Arc<SupervisorMessageSender>,
    ) -> Self {
        let (tx, rx) = channel(4096);

        spawn(async move {
            let receive_fut = async move {
                let receiver = CronSchedulerReceiver::new(proxy, pool, supervisor, rx).await?;
                receiver.receive().await
            };
            if let Err(e) = receive_fut.await {
                error!("{e}");
            }
        });

        Self { tx }
    }

    pub async fn add(
        &self,
        schedule: String,
        pipeline: String,
        variables: Option<HashMap<String, String>>,
        environment: Option<HashMap<String, String>>,
    ) -> Result<()> {
        self.tx
            .send(CronSchedulerMessage::Add {
                schedule,
                pipeline,
                variables,
                environment,
            })
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn remove(&self, pipeline: String) -> Result<()> {
        self.tx
            .send(CronSchedulerMessage::Remove { pipeline })
            .await
            .map_err(|e| anyhow!(e))
    }
}
