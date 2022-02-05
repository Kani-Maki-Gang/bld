#![allow(dead_code, unused_imports)]

use crate::high_avail::{AgentRequest, AgentResponse};
use crate::persist::ha_client_serial_responses::{
    self, HighAvailClientSerialResponses, InsertHighAvailClientSerialResponses,
};
use crate::persist::ha_client_status::{self, HighAvailClientStatus, InsertHighAvailClientStatus};
use crate::persist::ha_hard_state::{self, HighAvailHardState, InsertHighAvailHardState};
use crate::persist::ha_log::{self, HighAvailLog, InsertHighAvailLog, BLANK};
use crate::persist::ha_members::{self, HighAvailMembers, InsertHighAvailMembers};
use crate::persist::ha_members_after_consensus::{
    self, HighAvailMembersAfterConsensus, InsertHighAvailMembersAfterConsensus,
};
use crate::persist::ha_snapshot::{self, HighAvailSnapshot, InsertHighAvailSnapshot};
use crate::persist::ha_state_machine::{self, HighAvailStateMachine};
use anyhow::{anyhow, Result};
use async_raft::raft::{Entry, EntryPayload, MembershipConfig};
use async_raft::storage::{CurrentSnapshotData, HardState, InitialState, RaftStorage};
use async_raft::NodeId;
use async_trait::async_trait;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::{self, Display};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, error};

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

pub struct HighAvailStore {
    pool: Pool<ConnectionManager<SqliteConnection>>,
    id: NodeId,
    sm: RwLock<HighAvailStateMachine>,
    hs: RwLock<Option<HighAvailHardState>>,
    current_snapshot: RwLock<Option<HighAvailSnapshot>>,
}

impl HighAvailStore {
    pub fn new(
        pool: Pool<ConnectionManager<SqliteConnection>>,
        id: NodeId,
    ) -> anyhow::Result<Self> {
        let conn = pool.get()?;
        let sm = ha_state_machine::select_first(&conn)
            .or_else(|_| ha_state_machine::insert(&conn, 0))?;
        Ok(Self {
            pool,
            id,
            sm: RwLock::new(sm),
            hs: RwLock::new(None),
            current_snapshot: RwLock::new(None),
        })
    }
}

#[async_trait]
impl RaftStorage<AgentRequest, AgentResponse> for HighAvailStore {
    type Snapshot = Cursor<Vec<u8>>;
    type ShutdownError = ShutdownError;

    async fn get_membership_config(&self) -> anyhow::Result<MembershipConfig> {
        debug!("getting membership config");
        let conn = self.pool.get()?;
        ha_log::select_by_payload_type(&conn)
            .and_then(|log| {
                serde_json::from_str::<EntryPayload<AgentRequest>>(&log.payload)
                    .map(|p| match p {
                        EntryPayload::ConfigChange(cfg) => cfg.membership,
                        EntryPayload::SnapshotPointer(snap) => snap.membership,
                        _ => MembershipConfig::new_initial(self.id),
                    })
                    .map_err(|e| anyhow!(e))
            })
            .or_else(|_| Ok(MembershipConfig::new_initial(self.id)))
    }

    async fn get_initial_state(&self) -> anyhow::Result<InitialState> {
        debug!("getting initial state");
        let conn = self.pool.get()?;
        let membership = self.get_membership_config().await?;
        let sm = self.sm.read().await;
        match ha_hard_state::select_first(&conn) {
            Ok(hs) => {
                let lg = ha_log::select_first_by_date_created_desc(&conn).or_else(|_| {
                    ha_log::insert(&conn, InsertHighAvailLog::new(0, 0, BLANK, None))
                })?;
                Ok(InitialState {
                    last_log_index: lg.id as u64,
                    last_log_term: lg.term as u64,
                    last_applied_log: sm.last_applied_log as u64,
                    hard_state: hs.into(),
                    membership,
                })
            }
            Err(_) => {
                let new = InitialState::new_initial(self.id);
                ha_hard_state::insert(&conn, (&new.hard_state).into())?;
                Ok(new)
            }
        }
    }

    async fn save_hard_state(&self, hs: &HardState) -> anyhow::Result<()> {
        debug!("saving hard state");
        let conn = self.pool.get()?;
        let model = match &*self.hs.read().await {
            Some(x) => ha_hard_state::update(
                &conn,
                x.id,
                hs.current_term as i32,
                hs.voted_for.map(|i| i as i32),
            )
            .ok(),
            None => ha_hard_state::insert(&conn, hs.into()).ok(),
        };
        *self.hs.write().await = model;
        Ok(())
    }

    async fn get_log_entries(
        &self,
        start: u64,
        stop: u64,
    ) -> anyhow::Result<Vec<Entry<AgentRequest>>> {
        debug!("getting log entries");
        if start > stop {
            return Ok(vec![]);
        }
        let conn = self.pool.get()?;

        // TODO: make a query for this filtering.
        let logs = ha_log::select_between_ids(&conn, start as i32, stop as i32)?;
        let entries: Vec<Entry<AgentRequest>> = logs 
            .iter()
            .map(|l| serde_json::from_str::<Entry<AgentRequest>>(&l.payload))
            .flatten()
            .collect();
        if entries.len() as u64 == (stop - start) {
            Ok(entries)
        } else {
            Err(anyhow!("found log entries with invalid payloads"))
        }
    }

    async fn delete_logs_from(&self, start: u64, stop: Option<u64>) -> anyhow::Result<()> {
        debug!("deleting logs from: {} to {:?}", start, stop);
        if stop.as_ref().map(|stop| &start > stop).unwrap_or(false) {
            return Ok(());
        }
        let conn = self.pool.get()?;
        match stop.as_ref() {
            Some(stop) => ha_log::delete_by_ids(
                &conn,
                (start..*stop).map(|i| i as i32).collect::<Vec<i32>>(),
            ),
            None => ha_log::delete_from_id(&conn, start as i32),
        }
    }

    async fn append_entry_to_log(&self, entry: &Entry<AgentRequest>) -> anyhow::Result<()> {
        debug!("appending entries to log");
        let conn = self.pool.get()?;
        ha_log::insert(&conn, entry.into()).map(|_| ())
    }

    async fn replicate_to_log(&self, entries: &[Entry<AgentRequest>]) -> anyhow::Result<()> {
        debug!("replicating to log");
        let conn = self.pool.get()?;
        ha_log::insert_many(&conn, entries.iter().map(|e| e.into()).collect()).map(|_| ())
    }

    async fn apply_entry_to_state_machine(
        &self,
        index: &u64,
        data: &AgentRequest,
    ) -> anyhow::Result<AgentResponse> {
        debug!("applying entry to state machine");
        let id = data.id().parse::<i32>()?;
        let status = data.status().to_string();
        let conn = self.pool.get()?;

        let sm = self.sm.read().await;
        ha_state_machine::update(&conn, sm.id, *index as i32)?;

        if let Ok(csr) = ha_client_serial_responses::select_by_id(&conn, id) {
            if csr.serial as u64 == data.serial() {
                return Ok(AgentResponse::new(csr.response));
            }
        }

        let previous = match ha_client_status::select_by_id(&conn, id) {
            Ok(old_cs) => {
                ha_client_status::update(&conn, id, &status)?;
                Some(old_cs.status)
            }
            Err(_) => {
                let cs = InsertHighAvailClientStatus::new(id, sm.id, &status);
                ha_client_status::insert(&conn, cs)?;
                None
            }
        };

        let csr = InsertHighAvailClientSerialResponses::new(
            id,
            sm.id,
            data.serial() as i32,
            previous.as_deref(),
        );
        ha_client_serial_responses::insert(&conn, csr)?;

        Ok(AgentResponse::new(Some(status)))
    }

    async fn replicate_to_state_machine(
        &self,
        entries: &[(&u64, &AgentRequest)],
    ) -> anyhow::Result<()> {
        debug!("replicating to state machine");
        let conn = self.pool.get()?;
        let mut sm = self.sm.write().await;
        for (index, data) in entries {
            let id = data.id().parse::<i32>()?;
            let status = data.status();
            (*sm).last_applied_log = **index as i32;
            if let Ok(csr) = ha_client_serial_responses::select_by_id(&conn, id) {
                if csr.serial as u64 == data.serial() {
                    continue;
                }
            }

            let previous = match ha_client_status::select_by_id(&conn, id) {
                Ok(old_cs) => {
                    ha_client_status::update(&conn, id, status)?;
                    Some(old_cs.status)
                }
                Err(_) => {
                    let cs = InsertHighAvailClientStatus::new(id, sm.id, status);
                    ha_client_status::insert(&conn, cs)?;
                    None
                }
            };

            let csr = InsertHighAvailClientSerialResponses::new(
                id,
                sm.id,
                data.serial() as i32,
                previous.as_deref(),
            );
            ha_client_serial_responses::insert(&conn, csr)?;
        }
        ha_state_machine::update(&conn, sm.id, sm.last_applied_log)?;
        Ok(())
    }

    async fn do_log_compaction(&self) -> anyhow::Result<CurrentSnapshotData<Self::Snapshot>> {
        debug!("doing log compaction");
        let conn = self.pool.get()?;
        let sm = self.sm.read().await;
        let data = serde_json::to_vec(&*sm)?;

        let membership_config = ha_log::select_config_greater_than_id(&conn, sm.last_applied_log)
            .map(
                |l| match serde_json::from_str::<EntryPayload<AgentRequest>>(&l.payload) {
                    Ok(EntryPayload::ConfigChange(cfg)) => Some(cfg.membership),
                    _ => None,
                },
            )
            .unwrap_or_else(|_| Some(MembershipConfig::new_initial(self.id)))
            .unwrap();

        let term = ha_log::select_by_id(&conn, sm.last_applied_log)
            .map(|l| l.term)
            .map_err(|_| anyhow!(ERR_INCONSISTENT_LOG))?;
        let entry = Entry::<AgentRequest>::new_snapshot_pointer(
            sm.last_applied_log as u64,
            term as u64,
            "".into(),
            membership_config.clone(),
        );
        ha_log::insert(&conn, entry.into())?;

        let snapshot = ha_snapshot::insert(
            &conn,
            InsertHighAvailSnapshot::new(sm.last_applied_log, term, data),
        )?;

        let members = membership_config
            .members
            .iter()
            .map(|m| InsertHighAvailMembers::new(*m as i32, snapshot.id))
            .collect();
        ha_members::insert_many(&conn, members)?;

        let members_after_consensus =
            membership_config
                .members_after_consensus
                .as_ref()
                .map(|entries| {
                    entries
                        .iter()
                        .map(|mc| {
                            InsertHighAvailMembersAfterConsensus::new(*mc as i32, snapshot.id)
                        })
                        .collect()
                });
        if let Some(mc) = members_after_consensus {
            ha_members_after_consensus::insert_many(&conn, mc)?;
        }

        let snapshot_bytes = serde_json::to_vec(&snapshot)?;

        Ok(CurrentSnapshotData {
            term: term as u64,
            index: sm.last_applied_log as u64,
            membership: membership_config,
            snapshot: Box::new(Cursor::new(snapshot_bytes)),
        })
    }

    async fn create_snapshot(&self) -> anyhow::Result<(String, Box<Self::Snapshot>)> {
        debug!("creating snapshot");
        Ok((String::from(""), Box::new(Cursor::new(Vec::new()))))
    }

    async fn finalize_snapshot_installation(
        &self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Self::Snapshot>,
    ) -> anyhow::Result<()> {
        debug!("finalizing snapshot installation");
        let conn = self.pool.get()?;
        let new_snapshot =
            serde_json::from_slice::<HighAvailSnapshot>(snapshot.get_ref().as_slice())?;
        {
            let sm = self.sm.read().await;
            let membership_config =
                ha_log::select_config_greater_than_id(&conn, sm.last_applied_log)
                    .map(
                        |l| match serde_json::from_str::<EntryPayload<AgentRequest>>(&l.payload) {
                            Ok(EntryPayload::ConfigChange(cfg)) => Some(cfg.membership),
                            _ => None,
                        },
                    )
                    .unwrap_or_else(|_| Some(MembershipConfig::new_initial(self.id)))
                    .unwrap();
            let entry =
                Entry::<AgentRequest>::new_snapshot_pointer(index, term, id, membership_config);
            match &delete_through {
                Some(through) => ha_log::delete_until_id(&conn, *through as i32)?,
                None => ha_log::delete(&conn)?,
            }
            ha_log::insert(&conn, entry.into())?;
        }

        {
            let new_sm = serde_json::from_slice::<HighAvailStateMachine>(&new_snapshot.data)?;
            let mut sm = self.sm.write().await;
            *sm = new_sm;
        }

        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(new_snapshot);
        Ok(())
    }

    async fn get_current_snapshot(
        &self,
    ) -> anyhow::Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        debug!("getting current snapshot");
        match &*self.current_snapshot.read().await {
            Some(snapshot) => {
                let conn = self.pool.get()?;
                let reader = serde_json::to_vec(&snapshot)?;
                let members = ha_members::select(&conn, snapshot)?
                    .iter()
                    .map(|m| m.id as NodeId)
                    .collect();
                let members_after_consensus = Some(
                    ha_members_after_consensus::select(&conn, snapshot)?
                        .iter()
                        .map(|m| m.id as NodeId)
                        .collect(),
                );
                let current = CurrentSnapshotData {
                    index: snapshot.id as u64,
                    term: snapshot.term as u64,
                    membership: MembershipConfig {
                        members,
                        members_after_consensus,
                    },
                    snapshot: Box::new(Cursor::new(reader)),
                };
                debug!(
                    "index: {}, term: {}, membership: {:?}, snapshot: {:?}",
                    current.index, current.term, current.membership, current.snapshot
                );
                Ok(Some(current))
            }
            None => {
                debug!("None");
                Ok(None)
            }
        }
    }
}
