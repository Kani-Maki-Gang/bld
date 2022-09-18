use actix::Message;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, mem::size_of};

pub static SERVER: &str = "server";
pub static WORKER: &str = "worker";

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum ServerMessages {
    Ack,
    Enqueue {
        pipeline: String,
        run_id: String,
        variables: Option<String>,
        environment: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize, Message)]
#[rtype(result = "()")]
pub enum WorkerMessages {
    Ack,
    WhoAmI { pid: u32 },
    Completed,
}

#[derive(Serialize, Deserialize)]
pub enum UnixSocketMessage {
    ServerAck,
    ServerEnqueue {
        pipeline: String,
        run_id: String,
        variables: Option<String>,
        environment: Option<String>,
    },
    WorkerAck {
        pid: u32,
    },
    WorkerPing,
    WorkerExit,
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
        leftover: Option<&[u8]>,
        capacity: usize,
    ) -> anyhow::Result<(Option<Vec<UnixSocketMessage>>, Option<Vec<u8>>)> {
        if capacity == 0 {
            return Ok((None, None));
        }
        let mut current = 0;
        let mut messages = vec![];
        let mut current_leftover: Option<Vec<u8>> = None;

        let mut bytes_inner = VecDeque::new();
        if let Some(leftover) = leftover {
            for b in leftover {
                bytes_inner.push_back(b);
            }
        }
        for b in bytes.iter() {
            bytes_inner.push_back(b);
        }

        while current < capacity {
            if bytes_inner.len() <= size_of::<u32>() {
                current_leftover = Some(bytes_inner.iter().map(|e| **e).collect());
                break;
            }

            let mut len: [u8; 4] = [0; 4];
            for i in 0..4 {
                len[i] = *bytes_inner.pop_front().unwrap();
            }

            let actual_len: usize = u32::from_le_bytes(len.try_into()?).try_into()?;
            if bytes_inner.len() <= actual_len {
                let mut leftover = vec![];
                leftover.extend_from_slice(&len[..]);
                leftover.extend_from_slice(bytes);
                current_leftover = Some(leftover);
                break;
            }

            let mut message = vec![];
            for _ in 0..actual_len {
                message.push(*bytes_inner.pop_front().unwrap());
            }

            messages.push(serde_json::from_slice(&message[..]).map_err(|e| anyhow!(e))?);

            current += size_of::<u32>() + actual_len;
        }
        Ok((Some(messages), current_leftover))
    }
}
