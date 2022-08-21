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
    worker_pid: u32,
    stream: UnixStream,
    state: UnixSocketConnectionState,
}

impl UnixSocketWorkerReader {
    pub fn new(worker_pid: u32, stream: UnixStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            worker_pid,
            stream,
            state: UnixSocketConnectionState::Active,
        }
    }

    pub fn has_pid(&self, pid: u32) -> bool {
        self.worker_pid == pid
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
                    debug!(
                        "worker with pid: {:?} sent PING message from unix socket with id: {}",
                        self.worker_pid,
                        self.id
                    );
                }
                UnixSocketMessage::WorkerExit => {
                    self.set_state(UnixSocketConnectionState::Stopped);
                    debug!(
                        "worker with pid: {:?} sent EXIT message from unix socket with id: {}",
                        self.worker_pid,
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
