use std::collections::VecDeque;
use std::fs;

use chrono::Utc;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::trace;

/// The actor itself.
///
pub struct StateActor;

#[derive(Debug)]
pub enum StateMsg {
    /// Add a job ID to the queue.
    Add(usize),
    /// Remove a job ID to the queue.
    Remove(usize),
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
    /// Job Queue
    pub queue: VecDeque<usize>,
}

pub struct StateArgs;

#[ractor::async_trait]
impl Actor for StateActor {
    type Msg = StateMsg;
    type State = State;
    type Arguments = String;

    async fn pre_start(&self, myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        trace!("stateactor::pre_start({:?}", args);
        let data = fs::read_to_string(&args)?;
        let mut data: State = serde_json::from_str(&data)?;
        data.fname = args.clone();

        Ok(data)
    }

    async fn handle(&self, myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
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
