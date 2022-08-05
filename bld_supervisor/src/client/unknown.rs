use crate::base::{UnixSocketConnectionState, UnixSocketRead, UnixSocketState};
use tokio::net::UnixStream;

pub enum ResolvedType {
    Server,
    Worker(u32),
}

pub struct UnixSocketUnknownReader {
    stream: UnixStream,
    state: UnixSocketConnectionState,
}

impl UnixSocketUnknownReader {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            state: UnixSocketConnectionState::Active,
        }
    }
}

impl UnixSocketRead for UnixSocketUnknownReader {
    fn get_stream(&self) -> &UnixStream {
        &self.stream
    }
}

impl UnixSocketState for UnixSocketUnknownReader {
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

impl Into<UnixStream> for UnixSocketUnknownReader {
    fn into(self) -> UnixStream {
        self.stream
    }
}
