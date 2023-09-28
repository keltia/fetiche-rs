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
                tm: Utc::now().timestamp(),
                last: *data.queue.back().unwrap_or(&1),
                queue: data.queue.clone(),
            };
            let data = json!(*data).to_string();
            Ok(fs::write(self.state_file(), data)?)
        } else {
            trace!("Dirty not set");
        }
    }
}

/// UpdateState
///
#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct UpdateState(String, String);

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
            data.state[tag] = state.clone();
            data.dirty = true;
        }
        Ok(())
    }
}

/// Request information about the current state
///
#[derive(Debug, Message)]
#[rtype(result = "Result<StateInfo>")]
pub struct Info;

#[derive(Debug)]
pub struct StateInfo {
    /// Homedir
    pub workdir: PathBuf,
    /// Last sync
    pub tm: i64,
    /// Number of currently held state
    pub len: usize,
}

impl<A, M> MessageResponse<A, M> for StateInfo
where
    A: Actor,
    M: Message<Result = StateInfo>,
{
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(self, _ctx: &mut A::Context, tx: Option<OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            let _ = tx.send(self);
        }
    }
}

impl Handler<Info> for StateActor {
    type Result = Result<StateInfo>;

    /// Return a subset of the current state
    ///
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: Info, _ctx: &mut Self::Context) -> Self::Result {
        let inner = self.inner.read().unwrap();

        Ok(StateInfo {
            workdir: self.workdir.clone(),
            tm: inner.tm,
            len: inner.state.len(),
        })
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
