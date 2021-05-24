#![allow(dead_code)]

use crate::helpers::request::http_post;
use crate::high_avail::{Agent, AgentRequest};
use crate::types::BldError;
use anyhow::Result;
use async_raft::config::Config;
use async_raft::network::RaftNetwork;
use async_raft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use async_raft::NodeId;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct HighAvailRouter {
    config: Arc<Config>,
    agents: HashSet<Agent>,
}

impl HighAvailRouter {
    pub async fn new(config: Arc<Config>, agents: HashSet<Agent>) -> Result<Self> {
        Ok(Self { config, agents })
    }

    fn agent(&self, id: &NodeId) -> Result<&Agent> {
        let agent = self
            .agents
            .iter()
            .find(|a| &a.id() == id)
            .ok_or_else(|| BldError::Other(format!("no agent with node id: {} found", id)))?;
        Ok(agent)
    }

    fn post<T>(&self, sub_url: &str, target: NodeId, body: T) -> Result<String>
    where
        T: 'static + Serialize,
    {
        let agent = self.agent(&target)?;
        let sys = String::from("bld-raft-net");
        let url = format!("http://{}:{}{}", agent.host(), agent.port(), sub_url);
        Ok(http_post(sys, url, HashMap::new(), body))
    }
}

#[async_trait]
impl RaftNetwork<AgentRequest> for HighAvailRouter {
    async fn append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<AgentRequest>,
    ) -> Result<AppendEntriesResponse> {
        let res = self.post("/ha/appendEntries", target, rpc)?;
        dbg!(&res);
        Ok(serde_json::from_str(&res)?)
    }

    async fn install_snapshot(
        &self,
        target: NodeId,
        rpc: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse> {
        let res = self.post("/ha/installSnapshot", target, rpc)?;
        Ok(serde_json::from_str(&res)?)
    }

    async fn vote(&self, target: NodeId, rpc: VoteRequest) -> Result<VoteResponse> {
        let res = self.post("/ha/vote", target, rpc)?;
        Ok(serde_json::from_str(&res)?)
    }
}
