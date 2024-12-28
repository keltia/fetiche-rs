//! State actor
//!
//! This is for managing the state file on disk.
//!
//! Operations:
//! - Add a job to the queue
//! - Remove a job after completion
//! - Sync file on-disk
//!
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::ENGINE_PID;
use chrono::Utc;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, trace};

/// The actor itself.
///
pub struct StateActor;

#[derive(Debug)]
pub enum StateMsg {
    /// Add a job ID to the queue.
    Add(usize),
    /// Remove a job ID to the queue.
    Remove(usize),
    /// Get next available id.
    Next(RpcReplyPort<usize>),
    /// Save current PID to file.
    SavePid(PathBuf),
    /// Get current PID.
    GetPid(RpcReplyPort<u32>),
    /// Sync unto state file on disk.
    Sync,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct State {
    /// Our state file path.
    #[serde(skip_deserializing, skip_serializing)]
    pub fname: String,
    /// Timestamp of last sync
    pub tm: i64,
    /// Last job ID
    pub last: usize,
    /// Current PID.
    #[serde(skip_deserializing, skip_serializing)]
    pub pid: u32,
    /// Job Queue
    pub queue: VecDeque<usize>,
}

pub struct StateArgs;

#[ractor::async_trait]
impl Actor for StateActor {
    type Msg = StateMsg;
    type State = State;
    type Arguments = String;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("stateactor::pre_start({:?}", args);
        let data = fs::read_to_string(&args)?;
        let mut data: State = serde_json::from_str(&data)?;
        data.fname = args.clone();
        myself.send_interval(Duration::from_secs(30), || StateMsg::Sync);

        Ok(data)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            StateMsg::Add(id) => {
                trace!("stateactor::add({id})");

                state.queue.push_back(id);
            }
            StateMsg::Remove(id) => {
                trace!("stateactor::remove({id})");

                if let Ok(index) = state.queue.binary_search(&id) {
                    trace!("Found job {}", id);
                    state.queue.remove(index);
                    trace!("queue={:?}", state.queue);
                }
            }
            StateMsg::Next(sender) => {
                trace!("stateactor::next({})", state.last);

                sender.send(state.last + 1)?;
            }
            StateMsg::SavePid(dir) => {
                trace!("stateactor::savepid({:?})", dir);

                let pid = std::process::id();
                state.pid = pid;
                let pidfile = dir.join(ENGINE_PID);
                fs::write(&pidfile, format!("{pid}"))
                    .unwrap_or_else(|_| panic!("can not write {}", pidfile.to_string_lossy()));

                info!("PID {} written in {:?}", pid, pidfile);
            }
            StateMsg::GetPid(sender) => {
                trace!("stateactor::getpid()");

                sender.send(state.pid)?;
            }
            StateMsg::Sync => {
                trace!("stateactor::sync");

                state.tm = Utc::now().timestamp();
                let data = json!(state).to_string();
                return Ok(fs::write(&state.fname, data)?);
            }
        }
        Ok(())
    }
}
