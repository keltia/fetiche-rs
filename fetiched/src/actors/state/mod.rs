//! Keeping state in Fetiched as an actor.  Create with the workdir directory as parameter,
//! it will load the state file if present.
//!
//! API:
//!
//! - `Info`
//! - `Sync`
//! - `RegisterState`
//! - `UpdateState`
//!
//! - `AddJob`
//! - `RemoveJob`
//!

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use actix::dev::{MessageResponse, OneshotSender};
use actix::{Actor, Context, Handler, Message};
use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, trace};

use crate::response_for;
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

impl Handler<UpdateState> for StateActor {
    type Result = Result<()>;

    fn handle(&mut self, msg: UpdateState, ctx: &mut Self::Context) -> Self::Result {
        // Retrieve sub-system tag and data
        //
        let tag = msg.0;
        let state = msg.1;

        // Lock & update
        {
            let mut data = self.inner.write()?;
            data.systems[&tag] = state.clone();
            data.dirty = true;
        }
        Ok(())
    }
}

/// Request information about the current state, state is per sub-system
///
#[derive(Debug, Message)]
#[rtype(result = "String")]
pub struct Info(String);

impl Info {
    /// Helper constructor
    ///
    pub fn about(tag: &str) -> Self {
        Info(tag.to_string())
    }
}

impl Handler<Info> for StateActor {
    type Result = String;

    /// Return a subset of the current state
    ///
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: Info, _ctx: &mut Self::Context) -> Self::Result {
        // Retrieve sub-system tag
        //
        let tag = msg.0;
        let inner = self.inner.read().unwrap();
        inner.systems.get(&tag).ok_or("".to_string())
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

#[cfg(test)]
mod tests {
    #[actix_rt::test]
    async fn test_actor_state_info() -> Result<()> {
        let workdir = std::env::temp_dir();
        let s = StateActor::new(&workdir).start();

        // We started fresh
        let si = s.send(Info).await?;
        assert!(si.is_ok());
        let si = si.unwrap();
        assert_eq!(workdir, PathBuf::from(&si.workdir));
        assert_eq!(0, si.len);
        dbg!(&si);
        Ok(())
    }
}
