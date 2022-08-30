use crate::base::{UnixSocketConnectionState, UnixSocketRead, UnixSocketState};
use tokio::net::UnixStream;
use uuid::Uuid;

pub enum ResolvedType {
    Server,
    Worker(u32),
}

pub struct UnixSocketUnknownReader {
    id: Uuid,
    stream: UnixStream,
    state: UnixSocketConnectionState,
    leftovers: Option<Vec<u8>>,
}

impl UnixSocketUnknownReader {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            stream,
            state: UnixSocketConnectionState::Active,
            leftovers: None,
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id.clone()
    }
}

impl UnixSocketRead for UnixSocketUnknownReader {
    fn set_leftover(&mut self, leftover: Option<Vec<u8>>) {
        self.leftovers = leftover;
    }

    fn get_leftover(&self) -> Option<&Vec<u8>> {
        self.leftovers.as_ref()
    }

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
