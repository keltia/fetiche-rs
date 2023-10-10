//! Keeping state in Fetiched as an actor.  Create with the workdir directory as parameter,
//! it will load the state file if present.
//!
//! API:
//!
//! - `GetState`
//! - `Sync`
//! - `UpdateState`
//!

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use actix::dev::MessageResponse;
use actix::{Actor, Context, Handler, Message};
use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, trace};

pub use core::*;

mod core;

/// Main state data file, will be created in `basedir`.
pub(crate) const STATE_FILE: &str = "state";

// ---- Messages

/// `Sync` is the message sent regularly to save all the current state.  Can be sent as well to
/// indicate wish for immediate synchronisation.
///
#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct Sync;

impl Handler<Sync> for StateActor {
    type Result = Result<()>;

    /// Lock and save the current state in the default file
    ///
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, _msg: Sync, _ctx: &mut Self::Context) -> Self::Result {
        trace!("state::sync");
        let mut data = self.inner.write().unwrap();
        if data.dirty {
            *data = State {
                version: STATE_VERSION,
                tm: Utc::now().timestamp(),
                dirty: false,
                systems: data.systems.clone(),
            };
            let content = json!(*data).to_string();
            Ok(fs::write(self.state_file(), content)?)
        } else {
            trace!("Dirty not set");
            Ok(())
        }
    }
}

/// UpdateState
///
#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct UpdateState(pub String, pub String);

impl UpdateState {
    pub fn service(tag: &str, data: String) -> Self {
        Self(tag.to_string(), data.clone())
    }
}

impl Handler<UpdateState> for StateActor {
    type Result = Result<()>;

    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: UpdateState, _ctx: &mut Self::Context) -> Self::Result {
        // Retrieve sub-system tag and data
        //
        let tag = msg.0;
        let state = msg.1;

        // Lock & update
        {
            let mut data = self.inner.write().unwrap();
            dbg!(&data);
            data.systems.insert(tag, state.clone());
            data.dirty = true;
        }
        Ok(())
    }
}

/// Request information about the current state, state is per sub-system
///
#[derive(Debug, Message)]
#[rtype(result = "String")]
pub struct GetState(String);

impl GetState {
    /// Helper constructor
    ///
    pub fn about(tag: &str) -> Self {
        GetState(tag.to_string())
    }
}

impl Handler<GetState> for StateActor {
    type Result = String;

    /// Return a subset of the current state
    ///
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: GetState, _ctx: &mut Self::Context) -> Self::Result {
        // Retrieve sub-system tag
        //
        let tag = msg.0;
        trace!("getting {}", tag);
        let inner = self.inner.read().unwrap();
        match inner.systems.get(&tag) {
            Some(res) => res.to_string(),
            None => panic!("empty state"),
        }
    }
}

// ----- Actor

#[derive(Debug)]
pub struct StateActor {
    workdir: PathBuf,
    inner: Arc<RwLock<State>>,
}

impl Actor for StateActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("State is alive");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        info!("State is stopped");
    }
}

impl StateActor {
    /// Load state from a file or create a new one
    ///
    #[tracing::instrument]
    pub fn new(workdir: &PathBuf) -> Self {
        // Get homedir
        //
        let file = workdir.join(STATE_FILE);

        trace!("Loading state from {:?}.", file);

        let state = State::from(file).unwrap_or(State::new());
        Self {
            workdir: workdir.to_owned(),
            inner: Arc::new(RwLock::new(state)),
        }
    }

    /// Returns the path of the default state file in basedir
    ///
    pub fn state_file(&self) -> PathBuf {
        self.workdir.join(STATE_FILE)
    }
}
