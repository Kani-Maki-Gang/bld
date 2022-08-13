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

    fn activate(&mut self, worker: Arc<Mutex<PipelineWorker>>) {
        {
            let mut worker = worker.lock().unwrap();
            let _ = worker.spawn();
        }
        self.active.push(worker);
    }
}

impl Queue<Arc<Mutex<PipelineWorker>>> for WorkerQueue {
    /// Used to spawn the child process of the worker and add it to the active workers vector.
    fn enqueue(&mut self, item: Arc<Mutex<PipelineWorker>>) {
        if self.active.len() < self.capacity {
            self.activate(item);
        } else {
            self.backlog.push_back(item);
        }
    }

    /// This method will check for worker that have finished executing and will remove them from
    /// the active workers collection. It will pop the appropriate amount of workers from the
    /// backlog vector, spawn them and add them as active.
    fn refresh(&mut self) {
        self.active.retain(|w| {
            let mut w = w.lock().unwrap();
            !w.cleanup().is_ok() 
        });
        for _ in 0..(self.capacity - self.active.len()) {
            if let Some(worker) = self.backlog.pop_front() {
                self.activate(worker);
            }
        }
    }

    fn find(&mut self, id: u32) -> Option<Arc<Mutex<PipelineWorker>>> {
        self.active
            .iter()
            .find(|w| {
                let w = w.lock().unwrap();
                w.has_pid(id)
            })
            .map(|w| w.clone())
            .or_else(|| {
                self.backlog
                    .iter()
                    .find(|w| {
                        let w = w.lock().unwrap();
                        w.has_pid(id)
                    })
                    .map(|w| w.clone())
            })
    }
}
