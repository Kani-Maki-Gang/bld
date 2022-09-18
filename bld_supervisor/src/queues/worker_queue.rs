use crate::base::Queue;
use actix_web::web::Data;
use bld_core::database::pipeline_runs;
use bld_core::workers::PipelineWorker;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use std::collections::VecDeque;

/// The QueueManager is initialized with a capacity of active workers.
/// If there are more workers than the specified capacity, the queue manager
/// will add them to a backlog based on when they were enqueued.
pub struct WorkerQueue {
    capacity: usize,
    active: Vec<PipelineWorker>,
    backlog: VecDeque<PipelineWorker>,
    pool: Data<Pool<ConnectionManager<SqliteConnection>>>,
}

impl WorkerQueue {
    pub fn new(capacity: usize, pool: Data<Pool<ConnectionManager<SqliteConnection>>>) -> Self {
        Self {
            capacity,
            active: Vec::with_capacity(capacity),
            backlog: VecDeque::new(),
            pool,
        }
    }

    fn activate(&mut self, mut worker: PipelineWorker) -> anyhow::Result<()> {
        worker.spawn()?;
        self.active.push(worker);
        Ok(())
    }

    fn add_backlog(&mut self, worker: PipelineWorker) -> anyhow::Result<()> {
        let conn = self.pool.get()?;
        pipeline_runs::update_state(&conn, worker.get_run_id(), "queued")?;
        self.backlog.push_back(worker);
        Ok(())
    }
}

impl Queue<PipelineWorker> for WorkerQueue {
    /// Used to spawn the child process of the worker and add it to the active workers vector.
    fn enqueue(&mut self, item: PipelineWorker) -> anyhow::Result<()> {
        if self.active.len() < self.capacity {
            self.activate(item)?;
        } else {
            self.add_backlog(item)?;
        }
        Ok(())
    }

    /// This method will check for a worker that have finished executing and will remove them from
    /// the active workers collection. It will pop the appropriate amount of workers from the
    /// backlog vector, spawn them and add them as active.
    fn dequeue(&mut self, pid: u32) -> anyhow::Result<()> {
        self.active.retain_mut(|w| {
            let found = w
                .get_pid()
                .as_ref()
                .map(|wpid| pid == *wpid)
                .unwrap_or(false);
            if found {
                let _ = w.cleanup();
            }
            !found
        });
        for _ in 0..(self.capacity - self.active.len()) {
            if let Some(worker) = self.backlog.pop_front() {
                self.activate(worker)?;
            }
        }
        Ok(())
    }

    fn contains(&mut self, pid: u32) -> bool {
        self.active
            .iter()
            .find(|w| w.has_pid(pid))
            .or_else(|| self.backlog.iter().find(|w| w.has_pid(pid)))
            .is_some()
    }
}
