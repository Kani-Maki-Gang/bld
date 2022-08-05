use crate::base::Queue;
use bld_core::workers::PipelineWorker;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// The QueueManager is initialized with a capacity of active workers.
/// If there are more workers than the specified capacity, the queue manager
/// will add them to a backlog based on when they were enqueued.
pub struct WorkerQueue {
    capacity: usize,
    active: Vec<Arc<Mutex<PipelineWorker>>>,
    backlog: VecDeque<Arc<Mutex<PipelineWorker>>>,
}

impl WorkerQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            active: Vec::with_capacity(capacity),
            backlog: VecDeque::new(),
        }
    }

    /// Used to spawn the child process of the worker and add it to the active workers vector.
    fn activate(&mut self, worker: Arc<Mutex<PipelineWorker>>) {
        {
            let mut worker = worker.lock().unwrap();
            let _ = worker.spawn();
        }
        self.active.push(worker);
    }

    /// This method will check for worker that have finished executing and will remove them from
    /// the active workers collection. It will pop the appropriate amount of workers from the
    /// backlog vector, spawn them and add them as active.
    pub fn refresh(&mut self) {

    }
}

impl Queue<Arc<Mutex<PipelineWorker>>> for WorkerQueue {
    fn enqueue(&mut self, item: Arc<Mutex<PipelineWorker>>) {
        if self.active.len() < self.capacity {
            self.activate(item);
        } else {
            self.backlog.push_back(item);
        }
    }

    fn refresh(&mut self) {
        self.active.retain(|w| {
            let w = w.lock().unwrap();
            !w.has_stopped()
        });
        for _ in 0..(self.capacity - self.active.len()) {
            if let Some(worker) = self.backlog.pop_front() {
                self.activate(worker);
            }
        }
    }
}
