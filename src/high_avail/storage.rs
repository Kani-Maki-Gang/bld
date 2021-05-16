#![allow(dead_code)]

use crate::high_avail::{AgentRequest, AgentResponse};
use anyhow::Result;
use async_raft::raft::{Entry, EntryPayload, MembershipConfig};
use async_raft::storage::{CurrentSnapshotData, HardState, InitialState, RaftStorage};
use async_raft::NodeId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::{self, Display};
use std::io::Cursor;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

const ERR_INCONSISTENT_LOG: &str =
    "a query was received which was expecting data to be in place which does not exist in the log";

#[derive(Clone, Debug)]
pub enum ShutdownError {
    UnsafeStorageError,
}

impl Display for ShutdownError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsafe storage error")
    }
}

impl Error for ShutdownError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HighAvailStoreSnapshot {
    pub index: u64,
    pub term: u64,
    pub membership: MembershipConfig,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct HighAvailStoreStateMachine {
    pub last_applied_log: u64,
    pub client_serial_responses: HashMap<String, (u64, Option<String>)>,
    pub client_status: HashMap<String, String>,
}

pub struct HighAvailStore {
    id: NodeId,
    log: RwLock<BTreeMap<u64, Entry<AgentRequest>>>,
    sm: RwLock<HighAvailStoreStateMachine>,
    hs: RwLock<Option<HardState>>,
    current_snapshot: RwLock<Option<HighAvailStoreSnapshot>>,
}

impl HighAvailStore {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            log: RwLock::new(BTreeMap::new()),
            sm: RwLock::new(HighAvailStoreStateMachine::default()),
            hs: RwLock::new(None),
            current_snapshot: RwLock::new(None),
        }
    }

    pub async fn get_log(&self) -> RwLockWriteGuard<'_, BTreeMap<u64, Entry<AgentRequest>>> {
        self.log.write().await
    }

    pub async fn get_state_machine(&self) -> RwLockWriteGuard<'_, HighAvailStoreStateMachine> {
        self.sm.write().await
    }

    pub async fn read_hard_state(&self) -> RwLockReadGuard<'_, Option<HardState>> {
        self.hs.read().await
    }
}

#[async_trait]
impl RaftStorage<AgentRequest, AgentResponse> for HighAvailStore {
    type Snapshot = Cursor<Vec<u8>>;
    type ShutdownError = ShutdownError;

    async fn get_membership_config(&self) -> Result<MembershipConfig> {
        let log = self.log.read().await;
        let cfg_opt = log.values().rev().find_map(|entry| match &entry.payload {
            EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
            EntryPayload::SnapshotPointer(snap) => Some(snap.membership.clone()),
            _ => None,
        });
        Ok(match cfg_opt {
            Some(cfg) => cfg,
            None => MembershipConfig::new_initial(self.id),
        })
    }

    async fn get_initial_state(&self) -> Result<InitialState> {
        let membership = self.get_membership_config().await?;
        let mut hs = self.hs.write().await;
        let log = self.log.read().await;
        let sm = self.sm.read().await;
        match &mut *hs {
            Some(inner) => {
                let (last_log_index, last_log_term) = match log.values().rev().next() {
                    Some(log) => (log.index, log.term),
                    None => (0, 0),
                };
                let last_applied_log = sm.last_applied_log;
                Ok(InitialState {
                    last_log_index,
                    last_log_term,
                    last_applied_log,
                    hard_state: inner.clone(),
                    membership,
                })
            }
            None => {
                let new = InitialState::new_initial(self.id);
                *hs = Some(new.hard_state.clone());
                Ok(new)
            }
        }
    }

    async fn save_hard_state(&self, hs: &HardState) -> Result<()> {
        *self.hs.write().await = Some(hs.clone());
        Ok(())
    }

    async fn get_log_entries(&self, start: u64, stop: u64) -> Result<Vec<Entry<AgentRequest>>> {
        if start > stop {
            return Ok(vec![]);
        }
        let log = self.log.read().await;
        Ok(log.range(start..stop).map(|(_, val)| val.clone()).collect())
    }

    async fn delete_logs_from(&self, start: u64, stop: Option<u64>) -> Result<()> {
        if stop.as_ref().map(|stop| &start > stop).unwrap_or(false) {
            return Ok(());
        }
        let mut log = self.log.write().await;

        if let Some(stop) = stop.as_ref() {
            for key in start..*stop {
                log.remove(&key);
            }
            return Ok(());
        }
        log.split_off(&start);
        Ok(())
    }

    async fn append_entry_to_log(&self, entry: &Entry<AgentRequest>) -> Result<()> {
        let mut log = self.log.write().await;
        log.insert(entry.index, entry.clone());
        Ok(())
    }

    async fn replicate_to_log(&self, entries: &[Entry<AgentRequest>]) -> Result<()> {
        let mut log = self.log.write().await;
        for entry in entries {
            log.insert(entry.index, entry.clone());
        }
        Ok(())
    }

    async fn apply_entry_to_state_machine(
        &self,
        index: &u64,
        data: &AgentRequest,
    ) -> Result<AgentResponse> {
        let id = data.id().to_string();
        let status = data.status().to_string();
        let mut sm = self.sm.write().await;
        sm.last_applied_log = *index;
        if let Some((serial, res)) = sm.client_serial_responses.get(&id) {
            if serial == &data.serial() {
                return Ok(AgentResponse::new(res.clone()));
            }
        }
        let previous = sm.client_status.insert(id.clone(), status);
        sm.client_serial_responses
            .insert(id, (data.serial(), previous.clone()));
        Ok(AgentResponse::new(previous))
    }

    async fn replicate_to_state_machine(&self, entries: &[(&u64, &AgentRequest)]) -> Result<()> {
        let mut sm = self.sm.write().await;
        for (index, data) in entries {
            sm.last_applied_log = **index;
            if let Some((serial, _)) = sm.client_serial_responses.get(&data.id().to_string()) {
                if serial == &data.serial() {
                    continue;
                }
            }
            let previous = sm
                .client_status
                .insert(data.id().to_string(), data.status().to_string());
            sm.client_serial_responses
                .insert(data.id().to_string(), (data.serial(), previous.clone()));
        }
        Ok(())
    }

    async fn do_log_compaction(&self) -> Result<CurrentSnapshotData<Self::Snapshot>> {
        let (data, last_applied_log);
        {
            let sm = self.sm.read().await;
            data = serde_json::to_vec(&*sm)?;
            last_applied_log = sm.last_applied_log;
        }

        let membership_config;
        {
            let log = self.log.read().await;
            membership_config = log
                .values()
                .rev()
                .skip_while(|entry| entry.index > last_applied_log)
                .find_map(|entry| match &entry.payload {
                    EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| MembershipConfig::new_initial(self.id));
        }

        let snapshot_bytes: Vec<u8>;
        let term;
        {
            let mut log = self.log.write().await;
            let mut current_snapshot = self.current_snapshot.write().await;
            term = log
                .get(&last_applied_log)
                .map(|entry| entry.term)
                .ok_or_else(|| anyhow::anyhow!(ERR_INCONSISTENT_LOG))?;
            *log = log.split_off(&last_applied_log);
            log.insert(
                last_applied_log,
                Entry::new_snapshot_pointer(
                    last_applied_log,
                    term,
                    "".into(),
                    membership_config.clone(),
                ),
            );

            let snapshot = HighAvailStoreSnapshot {
                index: last_applied_log,
                term,
                membership: membership_config.clone(),
                data,
            };
            snapshot_bytes = serde_json::to_vec(&snapshot)?;
            *current_snapshot = Some(snapshot);
        }

        Ok(CurrentSnapshotData {
            term,
            index: last_applied_log,
            membership: membership_config.clone(),
            snapshot: Box::new(Cursor::new(snapshot_bytes)),
        })
    }

    async fn create_snapshot(&self) -> Result<(String, Box<Self::Snapshot>)> {
        Ok((String::from(""), Box::new(Cursor::new(Vec::new()))))
    }

    async fn finalize_snapshot_installation(
        &self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Self::Snapshot>,
    ) -> Result<()> {
        let new_snapshot: HighAvailStoreSnapshot =
            serde_json::from_slice(snapshot.get_ref().as_slice())?;
        {
            let mut log = self.log.write().await;
            let membership_config = log
                .values()
                .rev()
                .skip_while(|entry| entry.index > index)
                .find_map(|entry| match &entry.payload {
                    EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| MembershipConfig::new_initial(self.id));

            match &delete_through {
                Some(through) => {
                    *log = log.split_off(&(through + 1));
                }
                None => log.clear(),
            }
            log.insert(
                index,
                Entry::new_snapshot_pointer(index, term, id, membership_config),
            );
        }

        {
            let new_sm: HighAvailStoreStateMachine = serde_json::from_slice(&new_snapshot.data)?;
            let mut sm = self.sm.write().await;
            *sm = new_sm;
        }

        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(new_snapshot);
        Ok(())
    }

    async fn get_current_snapshot(&self) -> Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        match &*self.current_snapshot.read().await {
            Some(snapshot) => {
                let reader = serde_json::to_vec(&snapshot)?;
                Ok(Some(CurrentSnapshotData {
                    index: snapshot.index,
                    term: snapshot.term,
                    membership: snapshot.membership.clone(),
                    snapshot: Box::new(Cursor::new(reader)),
                }))
            }
            None => Ok(None),
        }
    }
}
