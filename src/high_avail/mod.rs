#![allow(dead_code)]

mod agent;
mod network;
mod storage;

pub use agent::*;
pub use network::*;
pub use storage::*;

use crate::config::BldConfig;
use crate::types::Result;
use async_raft::config::Config;
use async_raft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    Raft, VoteRequest, VoteResponse
};
use async_raft::error::RaftError;
use async_raft::NodeId;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::result::Result as StdResult;
use tokio::runtime::Runtime;
use uuid::Uuid;

pub type HighAvailRaft = Raft<AgentRequest, AgentResponse, HighAvailRouter, HighAvailStore>;

pub enum HighAvailThreadRequest {
    AppendEntries(AppendEntriesRequest<AgentRequest>),
    InstallSnapshot(InstallSnapshotRequest),
    Vote(VoteRequest),
}

pub enum HighAvailThreadResponse {
    AppendEntries(AppendEntriesResponse),
    InstallSnapshot(InstallSnapshotResponse),
    Vote(VoteResponse),
}

pub struct HighAvailThread {
    node_id: NodeId,
    raft_request_tx: Sender<(Uuid, HighAvailThreadRequest)>,
    raft_response_rx: Receiver<(Uuid, StdResult<HighAvailThreadResponse, RaftError>)>,
}

impl HighAvailThread {
    pub async fn new(config: &BldConfig) -> Result<Self> {
        let (agent, agents) = Self::agent_info(config).unwrap();
        let node_id = agent.id();
        let (raft_request_tx, raft_request_rx) = channel();
        let (raft_response_tx, raft_response_rx) = channel();
        // Creating a new thread in order to create the needed tokio runtime.
        // This cannot be done normally because this function is called from an actix_web runtime.
        thread::spawn(move || Self::raft_thread(agent, agents, raft_request_rx, raft_response_tx));
        Ok(Self { node_id, raft_request_tx, raft_response_rx })
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn raft_request_tx(&self) -> &Sender<(Uuid, HighAvailThreadRequest)> {
        &self.raft_request_tx
    }

    pub fn raft_response_rx(&self) -> &Receiver<(Uuid, StdResult<HighAvailThreadResponse, RaftError>)> {
        &self.raft_response_rx
    }

    fn agent_info(config: &BldConfig) -> Result<(Agent, HashSet<Agent>)> {
        let node_id = config.local.node_id.ok_or("node_id not found")?;
        let agent = Agent::new(node_id, &config.local.host, config.local.port);
        let mut agents = HashSet::new();
        for server in config.remote.servers.iter() {
            let node_id = server
                .node_id
                .ok_or(format!("server: {}, node_id not found", server.name))?;
            agents.insert(Agent::new(node_id, &server.host, server.port));
        }
        agents.insert(agent.clone());
        Ok((agent, agents))
    }

    fn raft_thread(
        agent: Agent, 
        agents: HashSet<Agent>, 
        raft_req_rx: Receiver<(Uuid, HighAvailThreadRequest)>, 
        raft_resp_tx: Sender<(Uuid, StdResult<HighAvailThreadResponse, RaftError>)>) {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let raft_config = Arc::new(Config::build("raft-group".into()).validate().unwrap());
            let ids: HashSet<NodeId> = agents.iter().map(|a| a.id()).collect();
            let network = Arc::new(
                HighAvailRouter::new(raft_config.clone(), agents)
                    .await
                    .unwrap(),
            );
            let store = Arc::new(HighAvailStore::new(agent.id()));
            let raft = Arc::new(HighAvailRaft::new(
                agent.id(),
                raft_config.clone(),
                network.clone(),
                store.clone(),
            ));
            raft.initialize(ids).await.unwrap();
            while let Ok(message) = raft_req_rx.recv() {
                match message {
                    (id, HighAvailThreadRequest::AppendEntries(req)) => {
                        let resp = raft
                            .append_entries(req)
                            .await
                            .map(HighAvailThreadResponse::AppendEntries);
                        let _ = raft_resp_tx.send((id, resp));
                    }
                    (id, HighAvailThreadRequest::InstallSnapshot(req)) => {
                        let resp = raft
                            .install_snapshot(req)
                            .await
                            .map(HighAvailThreadResponse::InstallSnapshot);
                        let _ = raft_resp_tx.send((id, resp));
                    }
                    (id, HighAvailThreadRequest::Vote(req)) => {
                        let resp = raft
                            .vote(req)
                            .await
                            .map(HighAvailThreadResponse::Vote);
                        let _ = raft_resp_tx.send((id, resp));
                    }
                }
            }
        });
    }
}

pub enum HighAvail {
    Enabled(Mutex<HighAvailThread>),
    Disabled,
}

impl HighAvail {
    pub async fn new(config: &BldConfig) -> Result<Self> {
        Ok(match config.local.ha_mode {
            true => Self::Enabled(Mutex::new(HighAvailThread::new(config).await?)),
            false => Self::Disabled,
        })
    }
}
