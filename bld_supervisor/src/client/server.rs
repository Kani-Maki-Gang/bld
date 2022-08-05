use crate::base::{
    UnixSocketConnectionState, UnixSocketHandle, UnixSocketMessage, UnixSocketRead, UnixSocketState,
};
use tokio::net::UnixStream;
use uuid::Uuid;

pub struct UnixSocketServerReader {
    id: Uuid,
    stream: UnixStream,
    state: UnixSocketConnectionState,
}

impl UnixSocketServerReader {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            id: Uuid::new_v4(),
            stream,
            state: UnixSocketConnectionState::Active,
        }
    }
}

impl UnixSocketRead for UnixSocketServerReader {
    fn get_stream(&self) -> &UnixStream {
        &self.stream
    }
}

impl UnixSocketHandle for UnixSocketServerReader {
    fn handle(&mut self, messages: Vec<UnixSocketMessage>) {
        for message in messages.iter() {
            if let UnixSocketMessage::ServerEnqueue {
                pipeline,
                run_id,
                variables,
                environment,
            } = message
            {

            }
        }
    }
}

impl UnixSocketState for UnixSocketServerReader {
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
