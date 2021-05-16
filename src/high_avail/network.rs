#![allow(dead_code)]

use crate::high_avail::{Agent, AgentRequest};
use anyhow::Result;
use async_raft::config::Config;
use async_raft::network::RaftNetwork;
use async_raft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use async_raft::NodeId;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;

pub struct HighAvailRouter {
    config: Arc<Config>,
    agents: HashSet<Agent>,
}

impl HighAvailRouter {
    pub fn new(config: Arc<Config>, agents: HashSet<Agent>) -> Self {
        Self { config, agents }
    }
}

#[async_trait]
impl RaftNetwork<AgentRequest> for HighAvailRouter {
    async fn append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<AgentRequest>,
    ) -> Result<AppendEntriesResponse> {
        println!(
            "called append_entries with target: {:?}, rpc: {:?}",
            &target, &rpc
        );
        Ok(AppendEntriesResponse {
            term: 0,
            success: false,
            conflict_opt: None,
        })
    }

    async fn install_snapshot(
        &self,
        target: NodeId,
        rpc: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse> {
        println!(
            "called install_snapshot with target: {:?}, rpc: {:?}",
            &target, &rpc
        );
        Ok(InstallSnapshotResponse { term: 0 })
    }

    async fn vote(&self, target: NodeId, rpc: VoteRequest) -> Result<VoteResponse> {
        println!("called vote with target: {:?}, rpc: {:?}", &target, &rpc);
        Ok(VoteResponse {
            term: 0,
            vote_granted: false,
        })
    }
}
