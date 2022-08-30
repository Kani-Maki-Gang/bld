use crate::base::Queue;
use bld_core::workers::PipelineWorker;
use std::collections::VecDeque;

/// The QueueManager is initialized with a capacity of active workers.
/// If there are more workers than the specified capacity, the queue manager
/// will add them to a backlog based on when they were enqueued.
pub struct WorkerQueue {
    capacity: usize,
    active: Vec<PipelineWorker>,
    backlog: VecDeque<PipelineWorker>,
}

impl WorkerQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            active: Vec::with_capacity(capacity),
            backlog: VecDeque::new(),
        }
    }

    fn activate(&mut self, mut worker: PipelineWorker) {
        let _ = worker.spawn();
        self.active.push(worker);
    }
}

impl Queue<PipelineWorker> for WorkerQueue {
    /// Used to spawn the child process of the worker and add it to the active workers vector.
    fn enqueue(&mut self, item: PipelineWorker) {
        if self.active.len() < self.capacity {
            self.activate(item);
        } else {
            self.backlog.push_back(item);
        }
    }

    /// This method will check for worker that have finished executing and will remove them from
    /// the active workers collection. It will pop the appropriate amount of workers from the
    /// backlog vector, spawn them and add them as active.
    fn dequeue(&mut self, pids: &[u32]) {
        self.active.retain_mut(|w| {
            let found = w
                .get_pid()
                .as_ref()
                .map(|pid| pids.contains(pid))
                .unwrap_or(false);
            if found {
                let _ = w.cleanup();
            }
            !found
        });
        for _ in 0..(self.capacity - self.active.len()) {
            if let Some(worker) = self.backlog.pop_front() {
                self.activate(worker);
            }
        }
    }

    fn contains(&mut self, pid: u32) -> bool {
        self.active
            .iter()
            .find(|w| w.has_pid(pid))
            .or_else(|| self.backlog.iter().find(|w| w.has_pid(pid)))
            .is_some()
    }
}
