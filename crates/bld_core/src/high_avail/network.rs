#![allow(dead_code)]

use crate::high_avail::{Agent, AgentRequest};
use anyhow::{anyhow, Result};
use async_raft::config::Config;
use async_raft::network::RaftNetwork;
use async_raft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use async_raft::NodeId;
use async_trait::async_trait;
use bld_utils::request;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::debug;

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
            .ok_or_else(|| anyhow!("no agent with node id: {} found", id))?;
        Ok(agent)
    }

    async fn post<T>(&self, sub_url: &str, target: NodeId, body: T) -> Result<String>
    where
        T: 'static + Serialize,
    {
        let agent = self.agent(&target)?;
        let url = format!("http://{}:{}{sub_url}", agent.host(), agent.port());
        request::post(url, HashMap::new(), body).await
    }
}

#[async_trait]
impl RaftNetwork<AgentRequest> for HighAvailRouter {
    async fn append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<AgentRequest>,
    ) -> Result<AppendEntriesResponse> {
        let res = self.post("/ha/appendEntries", target, rpc).await?;
        debug!(
            "sent append entries request to node: {} with result: {}",
            target, res
        );
        Ok(serde_json::from_str(&res)?)
    }

    async fn install_snapshot(
        &self,
        target: NodeId,
        rpc: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse> {
        let res = self.post("/ha/installSnapshot", target, rpc).await?;
        debug!(
            "sent install snapshot request to node: {} with result: {}",
            target, res
        );
        Ok(serde_json::from_str(&res)?)
    }

    async fn vote(&self, target: NodeId, rpc: VoteRequest) -> Result<VoteResponse> {
        let res = self.post("/ha/vote", target, rpc).await?;
        debug!("sent vote request to node: {} with result: {}", target, res);
        Ok(serde_json::from_str(&res)?)
    }
}
