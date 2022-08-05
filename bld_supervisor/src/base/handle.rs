use crate::base::UnixSocketMessage;

pub trait UnixSocketHandle {
    fn handle(&mut self, messages: Vec<UnixSocketMessage>);
}
