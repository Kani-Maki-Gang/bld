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
use async_raft::raft::Raft;
use async_raft::NodeId;
use std::collections::HashSet;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Runtime;

pub type HighAvailRaft = Raft<AgentRequest, AgentResponse, HighAvailRouter, HighAvailStore>;

pub struct HighAvailThread {
    node_id: NodeId,
    raft_tx: Sender<String>,
}

impl HighAvailThread {
    pub async fn new(config: &BldConfig) -> Result<Self> {
        let (agent, agents) = Self::agent_info(config).unwrap();
        let node_id = agent.id();
        let (raft_tx, raft_rx) = channel();
        // Creating a new thread in order to create the needed tokio runtime.
        // This cannot be done normally because this function is called from an actix_web runtime.
        thread::spawn(move || {
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
                while let Ok(message) = raft_rx.recv() {
                    println!("{}", message);
                }
            });
        });
        Ok(Self { node_id, raft_tx })
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
