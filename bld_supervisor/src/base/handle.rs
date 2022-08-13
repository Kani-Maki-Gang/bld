use crate::base::{Queue, UnixSocketMessage};
use bld_core::workers::PipelineWorker;
use std::sync::{Arc, Mutex};

pub trait UnixSocketHandle {
    fn handle<Q>(&mut self, queue: Arc<Mutex<Q>>, messages: Vec<UnixSocketMessage>)
    where
        Q: Queue<Arc<Mutex<PipelineWorker>>>;
}
