use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::mem::size_of;

#[derive(Serialize, Deserialize)]
pub enum UnixSocketMessage {
    Ping { pid: u32 },
    Exit { pid: u32 },
}

impl UnixSocketMessage {
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let message = serde_json::to_vec(self).map_err(|e| anyhow!(e))?;
        let length: u32 = message.len().try_into()?;
        let mut bytes = vec![];
        for byte in length.to_le_bytes() {
            bytes.push(byte);
        }
        for byte in message {
            bytes.push(byte);
        }
        Ok(bytes)
    }

    pub fn from_bytes(
        bytes: &mut &[u8],
        capacity: usize,
    ) -> anyhow::Result<Option<Vec<UnixSocketMessage>>> {
        if capacity == 0 {
            return Ok(None);
        }
        let mut current = 0;
        let mut messages = vec![];
        while current < capacity {
            let (len, rest) = bytes.split_at(size_of::<u32>());
            *bytes = rest;

            let len: usize = u32::from_le_bytes(len.try_into()?).try_into()?;
            let (message, rest) = bytes.split_at(len);
            *bytes = rest;

            messages.push(serde_json::from_slice(message).map_err(|e| anyhow!(e))?);

            current += size_of::<u32>() + len;
        }
        Ok(Some(messages))
    }
}
