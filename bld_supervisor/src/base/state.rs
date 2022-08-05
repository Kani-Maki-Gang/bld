pub enum UnixSocketConnectionState {
    Active,
    Stopped,
}

pub trait UnixSocketState {
    fn set_state(&mut self, state: UnixSocketConnectionState);
    fn get_state(&self) -> &UnixSocketConnectionState;
    fn has_stopped(&self) -> bool;
}
