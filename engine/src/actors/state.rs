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
use std::path::PathBuf;

use crate::{ENGINE_PG, ENGINE_PID, StateError};
use chrono::Utc;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, pg};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;
use tracing::{error, info, trace, warn};

/// The main state data file will be created in `basedir`.
pub(crate) const STATE_FILE: &str = "state";

/// The actor itself.
///
pub struct StateActor;

/// Messages handled by the `StateActor`.
///
/// The `StateMsg` enum defines various requests that can be sent to the `StateActor` for
/// managing and interacting with the state.
///
/// Variants:
/// - `Add(usize)`: Adds a job ID to the queue.
/// - `Remove(usize)`: Removes a job ID from the queue after completion.
/// - `Next(RpcReplyPort<usize>)`: Retrieves the next available job ID by incrementing the last job ID.
/// - `GetPid(RpcReplyPort<u32>)`: Fetches the PID of the current running state actor.
/// - `Sync`: Synchronizes the current state to the disk (stored in the state file).
///
#[derive(Debug)]
pub enum StateMsg {
    /// Add a job ID to the waiting queue.
    Submit(usize),
    /// From waiting to running queue.
    Run(usize),
    /// From running to finished queue
    Finished(usize),
    /// Wrapping up
    Remove(usize),
    /// Last used id.
    Last(RpcReplyPort<usize>),
    /// Get current PID.
    GetPid(RpcReplyPort<u32>),
    /// Sync unto state file on disk.
    Sync,
}

/// Represents the current state of the `StateActor` actor.
///
/// This structure defines the persistent state data that is managed by the
/// `StateActor`.
///
/// Fields:
/// - `fname`: A [`PathBuf`] representing the file path where the state data
///   is stored on disk. This field is excluded from serialization.
/// - `tm`: A timestamp (`i64`) of the last time the state was synchronized with
///   the disk. Stored in UTC.
/// - `dirty`: A boolean registering that data has been written, but not synced
/// - `last`: The last processed job ID (`usize`).
/// - `pid`: The process ID (`u32`) of the running instance. This field is
///   excluded from serialization.
/// - `queue`: A queue (`VecDeque<usize>`) storing the IDs of pending jobs to
///   be processed.
///
/// Implements:
/// - Derived traits: `Clone`, `Debug`, `Deserialize`, `Serialize`
///
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct State {
    /// Our state file path.
    #[serde(skip_deserializing, skip_serializing)]
    pub fname: PathBuf,
    /// Timestamp of last sync
    pub tm: i64,
    #[serde(skip_deserializing)]
    pub dirty: bool,
    /// Last job ID
    pub last: usize,
    /// Current PID, not synced because it is in the PID file.
    #[serde(skip_deserializing, skip_serializing)]
    pub pid: u32,
    /// Job Queues -- at startup, queue is empty, nothing is running.
    pub waiting: VecDeque<usize>,
    pub running: VecDeque<usize>,
    pub finished: VecDeque<usize>,
}

/// This is version 1 of the state file
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct State1 {
    /// Our state file path.
    #[serde(skip_deserializing, skip_serializing)]
    pub fname: PathBuf,
    /// Timestamp of last sync
    pub tm: i64,
    #[serde(skip_deserializing)]
    pub dirty: bool,
    /// Last job ID
    pub last: usize,
    /// Current PID, not synced because it is in the PID file.
    #[serde(skip_deserializing, skip_serializing)]
    pub pid: u32,
    /// Job Queues -- at startup, queue is empty, nothing is running.
    pub queue: VecDeque<usize>,
}

#[ractor::async_trait]
impl Actor for StateActor {
    type Msg = StateMsg;
    type State = State;
    type Arguments = PathBuf;

    /// Prepares the `StateActor` when it starts, initializing its state from a file or setting up new state data.
    ///
    /// This `pre_start` function is invoked before the `StateActor` starts processing messages.
    ///
    /// # Arguments
    ///
    /// - `myself`: A reference to the actor itself, used for messaging and lifecycle handling.
    /// - `args`: A [`PathBuf`] representing the base directory where the state file resides.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// - The initialized [`State`] struct, which represents the actor's state.
    /// - Or an [`ActorProcessingErr`] in case of an error during initialization, such as file read or JSON parse failure.
    ///
    /// # Behavior
    /// - Reads the state from a file (`state`) in the given `args` directory.
    /// - Updates the state with the current process ID (`pid`).
    /// - Writes the process ID to a `pid` file in the same directory.
    /// - Schedules a periodic synchronization (`Sync`) message to itself every 30 seconds.
    /// - Joins the actor to a process group (`ENGINE_PG`) for cluster-related operations.
    ///
    /// # Panics
    ///
    /// This function panics if it fails to write the `pid` file to disk.
    ///
    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let basedir = args.clone();
        let fname = basedir.join(STATE_FILE);

        let data = fs::read_to_string(&fname).await?;

        // Read the `state` file.  If we can't read it as `State`, try with the previous
        // version.  If it succeeds, regenerate a new one.
        //
        let mut data: State = match serde_json::from_str(&data) {
            Ok(state) => state,
            Err(_) => {
                let st: State1 = match serde_json::from_str(&data) {
                    Ok(st) => {
                        warn!("Previous version of the state file detected, resetting.");
                        st
                    }
                    Err(e) => {
                        error!(
                            "Impossible to load state file in {:?}: {}",
                            fname,
                            e.to_string()
                        );
                        return Err(StateError::UnrecognizedFile(
                            fname.to_string_lossy().to_string(),
                        )
                        .into());
                    }
                };
                // returns a new one using previous data
                //
                State {
                    last: st.last,
                    ..Default::default()
                }
            }
        };

        // Reset everything except the last pid we just loaded
        //
        data.fname = fname;
        data.pid = std::process::id();
        data.waiting = VecDeque::new();
        data.running = VecDeque::new();
        data.finished = VecDeque::new();
        data.dirty = false;

        let pidfile = basedir.join(ENGINE_PID);
        fs::write(&pidfile, format!("{}", data.pid))
            .await
            .unwrap_or_else(|_| panic!("can not write {}", pidfile.to_string_lossy()));
        info!("PID {} written in {:?}", data.pid, pidfile);

        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);

        Ok(data)
    }

    /// Handles incoming messages for the `StateActor`.
    ///
    /// This method processes various message types defined in the `StateMsg` enum.
    /// It updates the actor's state, interacts with the job queue, and responds to requests.
    ///
    /// # Arguments
    /// - `_myself`: A reference to the actor itself, which can be used to send or schedule messages.
    /// - `message`: An instance of `StateMsg` representing the message or request to process.
    /// - `state`: A mutable reference to the `State` managed by the `StateActor`.
    ///
    /// # Returns
    /// On successful processing, this method returns `Ok(())`. Any errors encountered
    /// during processing will be returned as an `ActorProcessingErr`.
    ///
    /// # Message Processing
    /// - `StateMsg::Add(usize)`: Adds a job ID to the job queue.
    /// - `StateMsg::Remove(usize)`: Removes a specific job ID from the job queue, if found.
    /// - `StateMsg::Last(RpcReplyPort<usize>)`: Returns the last used ID.
    /// - `StateMsg::Next(RpcReplyPort<usize>)`: Sends back the next available job ID.
    /// - `StateMsg::GetPid(RpcReplyPort<u32>)`: Sends back the current actor's process ID.
    /// - `StateMsg::Sync`: Synchronizes the current state to disk, updating the timestamp.
    ///
    #[tracing::instrument(skip(self, _myself))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            StateMsg::Submit(id) => {
                trace!("stateactor::add({id})");

                state.waiting.push_back(id);
                state.last = id;
                state.dirty = true;

                trace!("waiting={:?}", state.waiting);
                trace!("last={:?}", state.last);
            }
            StateMsg::Run(id) => {
                trace!("stateactor::run({id})");

                if let Ok(index) = state.waiting.binary_search(&id) {
                    trace!("Found job {}", id);
                    state.waiting.remove(index);
                    trace!("queue={:?}", state.waiting);
                    state.running.push_back(id);
                    state.dirty = true;
                }

                trace!("waiting={:?}", state.waiting);
                trace!("running={:?}", state.running);
                trace!("last={:?}", state.last);
            }
            StateMsg::Finished(id) => {
                trace!("stateactor::finish({id})");

                if let Ok(index) = state.running.binary_search(&id) {
                    trace!("Found job {}", id);
                    state.running.remove(index);
                    trace!("queue={:?}", state.finished);
                    state.finished.push_back(id);
                    state.dirty = true;
                }

                trace!("running={:?}", state.running);
                trace!("finished={:?}", state.finished);
                trace!("last={:?}", state.last);
            }
            StateMsg::Remove(id) => {
                trace!("stateactor::remove({id})");

                if let Ok(index) = state.finished.binary_search(&id) {
                    trace!("Found job {}", id);
                    state.finished.remove(index);
                    trace!("queue={:?}", state.finished);
                    state.dirty = true;
                }
            }
            StateMsg::Last(sender) => {
                trace!("stateactor::last({})", state.last);

                sender.send(state.last)?;
            }
            StateMsg::GetPid(sender) => {
                trace!("stateactor::getpid()");

                sender.send(state.pid)?;
            }
            StateMsg::Sync => {
                trace!("stateactor::sync");

                state.tm = Utc::now().timestamp();
                let data = json!(state).to_string();
                let _ = fs::write(&state.fname, data).await?;
                state.dirty = false;
            }
        }
        Ok(())
    }
}
