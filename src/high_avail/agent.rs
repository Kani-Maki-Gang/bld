#![allow(dead_code)]

use async_raft::{AppData, AppDataResponse, NodeId};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Eq, Serialize, Deserialize, Debug, Clone)]
pub struct Agent {
    id: NodeId,
    host: String,
    port: i64,
}

impl Agent {
    pub fn new(id: NodeId, host: &str, port: i64) -> Self {
        Self {
            id,
            host: host.to_string(),
            port,
        }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> i64 {
        self.port
    }
}

impl PartialEq for Agent {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.host == other.host && self.port == other.port
    }
}

impl Hash for Agent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.host.hash(state);
        self.port.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentRequest {
    id: String,
    serial: u64,
    status: String,
}

impl AgentRequest {
    pub fn new(id: &str, serial: u64, status: &str) -> Self {
        Self {
            id: id.to_string(),
            serial,
            status: status.to_string(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn serial(&self) -> u64 {
        self.serial
    }

    pub fn status(&self) -> &str {
        &self.status
    }
}

impl AppData for AgentRequest {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentResponse {
    data: Option<String>,
}

impl AgentResponse {
    pub fn new(data: Option<String>) -> Self {
        Self { data }
    }

    pub fn _data(&self) -> &Option<String> {
        &self.data
    }
}

impl AppDataResponse for AgentResponse {}
