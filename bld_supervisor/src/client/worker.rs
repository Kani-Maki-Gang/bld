use crate::base::{
    UnixSocketConnectionState, UnixSocketHandle, UnixSocketMessage, UnixSocketRead, UnixSocketState,
};
use bld_core::workers::PipelineWorker;
use tokio::net::UnixStream;
use tracing::debug;
use uuid::Uuid;

pub struct UnixSocketWorkerReader {
    id: Uuid,
    worker: PipelineWorker,
    stream: UnixStream,
    state: UnixSocketConnectionState,
}

impl UnixSocketWorkerReader {
    pub fn new(worker: PipelineWorker, stream: UnixStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            stream,
            state: UnixSocketConnectionState::Active,
            worker,
        }
    }

    pub fn has_pid(&self, pid: u32) -> bool {
        self.worker.get_pid().map(|id| id == pid).unwrap_or(false)
    }
}

impl UnixSocketRead for UnixSocketWorkerReader {
    fn get_stream(&self) -> &UnixStream {
        &self.stream
    }
}

impl UnixSocketHandle for UnixSocketWorkerReader {
    fn handle(&mut self, messages: Vec<UnixSocketMessage>) {
        for message in messages.iter() {
            match message {
                UnixSocketMessage::WorkerPing => {
                    debug!(
                        "worker with pid: {:?} sent PING message from unix socket with id: {}",
                        self.worker.get_pid(),
                        self.id
                    );
                }
                UnixSocketMessage::WorkerExit => {
                    debug!(
                        "worker with pid: {:?} sent EXIT message from unix socket with id: {}",
                        self.worker.get_pid(),
                        self.id
                    );
                    self.set_state(UnixSocketConnectionState::Stopped);
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
