use crate::base::{
    Queue, UnixSocketConnectionState, UnixSocketHandle, UnixSocketMessage, UnixSocketRead,
    UnixSocketState,
};
use bld_core::workers::PipelineWorker;
use std::sync::{Arc, Mutex};
use tokio::net::UnixStream;
use tracing::debug;
use uuid::Uuid;

pub struct UnixSocketWorkerReader {
    id: Uuid,
    worker: Arc<Mutex<PipelineWorker>>,
    stream: UnixStream,
    state: UnixSocketConnectionState,
}

impl UnixSocketWorkerReader {
    pub fn new(worker: Arc<Mutex<PipelineWorker>>, stream: UnixStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            stream,
            state: UnixSocketConnectionState::Active,
            worker,
        }
    }

    pub fn has_pid(&self, pid: u32) -> bool {
        let worker = self.worker.lock().unwrap();
        worker.has_pid(pid)
    }
}

impl UnixSocketRead for UnixSocketWorkerReader {
    fn get_stream(&self) -> &UnixStream {
        &self.stream
    }
}

impl UnixSocketHandle for UnixSocketWorkerReader {
    fn handle<Q>(&mut self, _queue: Arc<Mutex<Q>>, messages: Vec<UnixSocketMessage>)
    where
        Q: Queue<Arc<Mutex<PipelineWorker>>>,
    {
        for message in messages.iter() {
            match message {
                UnixSocketMessage::WorkerPing => {
                    let worker = self.worker.lock().unwrap();
                    debug!(
                        "worker with pid: {:?} sent PING message from unix socket with id: {}",
                        worker.get_pid(),
                        self.id
                    );
                }
                UnixSocketMessage::WorkerExit => {
                    self.set_state(UnixSocketConnectionState::Stopped);
                    let worker = self.worker.lock().unwrap();
                    debug!(
                        "worker with pid: {:?} sent EXIT message from unix socket with id: {}",
                        worker.get_pid(),
                        self.id
                    );
                }
                _ => {}
            }
        }
    }
}

impl UnixSocketState for UnixSocketWorkerReader {
    fn set_state(&mut self, state: UnixSocketConnectionState) {
        self.state = state;
    }

    fn get_state(&self) -> &UnixSocketConnectionState {
        &self.state
    }

    fn has_stopped(&self) -> bool {
        match self.state {
            UnixSocketConnectionState::Stopped => true,
            _ => false,
        }
    }
}
