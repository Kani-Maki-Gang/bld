#![allow(dead_code)]

use crate::helpers::request::http_post;
use crate::high_avail::{Agent, AgentRequest};
use anyhow::anyhow;
use async_raft::config::Config;
use async_raft::network::RaftNetwork;
use async_raft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use async_raft::NodeId;
use async_trait::async_trait;
use tracing::debug;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct HighAvailRouter {
    config: Arc<Config>,
    agents: HashSet<Agent>,
}

impl HighAvailRouter {
    pub async fn new(config: Arc<Config>, agents: HashSet<Agent>) -> anyhow::Result<Self> {
        Ok(Self { config, agents })
    }

    fn agent(&self, id: &NodeId) -> anyhow::Result<&Agent> {
        let agent = self
            .agents
            .iter()
            .find(|a| &a.id() == id)
            .ok_or_else(|| anyhow!("no agent with node id: {} found", id))?;
        Ok(agent)
    }

    fn post<T>(&self, sub_url: &str, target: NodeId, body: T) -> anyhow::Result<String>
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
    ) -> anyhow::Result<AppendEntriesResponse> {
        let res = self.post("/ha/appendEntries", target, rpc)?;
        debug!("sent append entries request to node: {} with result: {}", target, res);
        Ok(serde_json::from_str(&res)?)
    }

    async fn install_snapshot(
        &self,
        target: NodeId,
        rpc: InstallSnapshotRequest,
    ) -> anyhow::Result<InstallSnapshotResponse> {
        let res = self.post("/ha/installSnapshot", target, rpc)?;
        debug!("sent install snapshot request to node: {} with result: {}", target, res);
        Ok(serde_json::from_str(&res)?)
    }

    async fn vote(&self, target: NodeId, rpc: VoteRequest) -> anyhow::Result<VoteResponse> {
        let res = self.post("/ha/vote", target, rpc)?;
        debug!("sent vote request to node: {} with result: {}", target, res);
        Ok(serde_json::from_str(&res)?)
    }
}
