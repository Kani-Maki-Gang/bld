use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub enum UnixSocketMessage {
    Ping,
    Exit
}
