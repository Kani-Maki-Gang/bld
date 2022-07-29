use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum UnixSocketMessage {
    Ping { pid: u32 },
    Exit { pid: u32 },
}
