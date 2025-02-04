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

use chrono::Utc;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, trace};

use crate::{ENGINE_PG, ENGINE_PID};

/// Main state data file, will be created in `basedir`.
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
    /// Add a job ID to the queue.
    Add(usize),
    /// Remove a job ID to the queue.
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
/// - `last`: The last processed job ID (`usize`).
/// - `pid`: The process ID (`u32`) of the running instance. This field is
///   excluded from serialization.
/// - `queue`: A queue (`VecDeque<usize>`) storing the IDs of pending jobs to
///   be processed.
///
/// Implements:
/// - Derived traits: `Clone`, `Debug`, `Deserialize`, `Serialize`
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct State {
    /// Our state file path.
    #[serde(skip_deserializing, skip_serializing)]
    pub fname: PathBuf,
    /// Timestamp of last sync
    pub tm: i64,
    /// Last job ID
    pub last: usize,
    /// Current PID, not synced because it is in the PID file.
    #[serde(skip_deserializing, skip_serializing)]
    pub pid: u32,
    /// Job Queue -- at startup, queue is empty, nothing is running.
    #[serde(skip_deserializing)]
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
    #[tracing::instrument(skip(self))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("stateactor::pre_start({:?}", args);

        let basedir = args.clone();
        let fname = basedir.join(STATE_FILE);

        let data = fs::read_to_string(&fname)?;
        let mut data: State = serde_json::from_str(&data)?;

        data.fname = fname;
        data.pid = std::process::id();
        data.queue = VecDeque::new();

        let pidfile = basedir.join(ENGINE_PID);
        fs::write(&pidfile, format!("{}", data.pid))
            .unwrap_or_else(|_| panic!("can not write {}", pidfile.to_string_lossy()));
        info!("PID {} written in {:?}", data.pid, pidfile);
        myself.send_interval(Duration::from_secs(30), || StateMsg::Sync);

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
            StateMsg::Add(id) => {
                trace!("stateactor::add({id})");

                state.queue.push_back(id);
                state.last = id;

                trace!("queue={:?}", state.queue);
                trace!("last={:?}", state.last);
            }
            StateMsg::Remove(id) => {
                trace!("stateactor::remove({id})");

                if let Ok(index) = state.queue.binary_search(&id) {
                    trace!("Found job {}", id);
                    state.queue.remove(index);
                    trace!("queue={:?}", state.queue);
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
                return Ok(fs::write(&state.fname, data)?);
            }
        }
        Ok(())
    }
}
