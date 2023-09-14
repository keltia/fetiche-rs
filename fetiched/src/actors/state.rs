//! Keeping state in Fetiched as an actor.  Create with the workdir directory as parameter,
//! it will load the state file if present.
//!
//! API:
//!
//! - `Info`
//! - `Sync`
//!
//! - `AddJob`
//! - `RemoveJob`
//!

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use actix::dev::{MessageResponse, OneshotSender};
use actix::{Actor, Context, Handler, Message};
use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, trace};

/// Main state data file, will be created in `basedir`.
pub(crate) const STATE_FILE: &str = "state";

// ---- Messages

/// `Sync` is the message sent regularly to save all the current state.  Can be called as well to
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
        *data = State {
            tm: Utc::now().timestamp(),
            last: *data.queue.back().unwrap_or(&1),
            queue: data.queue.clone(),
        };
        let data = json!(*data).to_string();
        Ok(fs::write(self.state_file(), data)?)
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
    pub workdir: String,
    /// Last sync
    pub tm: i64,
    /// Queue size
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
            workdir: self.home.to_string(),
            tm: inner.tm,
            len: inner.queue.len(),
        })
    }
}

/// Add a job ID to the current job queue
///
#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct AddJob(usize);

impl Handler<AddJob> for StateActor {
    type Result = Result<()>;

    /// Add the specified job ID to the end of the queue
    ///
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: AddJob, _ctx: &mut Self::Context) -> Self::Result {
        let mut inner = self.inner.write().unwrap();
        inner.queue.push_back(msg.0);
        Ok(())
    }
}

/// Remove the specified job from the job queue
///
#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct RemoveJob(usize);

impl Handler<RemoveJob> for StateActor {
    type Result = Result<()>;

    /// Perform a binary search on the job queue (job id are always incrementing) and remove said
    /// job (done or cancelled, etc.).
    ///
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: RemoveJob, _ctx: &mut Self::Context) -> Self::Result {
        trace!("state::remove_job({})", msg.0);

        let id = msg.0;
        let mut inner = self.inner.write().unwrap();
        if let Ok(index) = inner.queue.binary_search(&id) {
            trace!("Found job {}", id);
            inner.queue.remove(index);
            trace!("queue={:?}", inner.queue);
        }
        Ok(())
    }
}

// ----- Actor

#[derive(Debug)]
pub struct StateActor {
    home: String,
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
    #[tracing::instrument]
    pub fn new(workdir: &str) -> Self {
        // Get homedir
        //
        let file = Path::new(workdir).join(STATE_FILE);

        trace!("Loading state from {}.", file.to_string_lossy());

        let state = State::from(file).unwrap_or(State::new());
        Self {
            home: workdir.to_owned(),
            inner: Arc::new(RwLock::new(state)),
        }
    }

    /// Returns the path of the default state file in basedir
    ///
    pub fn state_file(&self) -> PathBuf {
        Path::new(&self.home).join(STATE_FILE)
    }
}

/// Register the state of the running `Engine`.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
struct State {
    /// Timestamp
    pub tm: i64,
    /// Last job ID
    pub last: usize,
    /// Job Queue
    pub queue: VecDeque<usize>,
}

impl State {
    /// Create an clean and empty state
    ///
    pub fn new() -> Self {
        State {
            tm: Utc::now().timestamp(),
            last: 0,
            queue: VecDeque::<usize>::new(),
        }
    }

    /// Read our JSON file
    ///
    #[tracing::instrument]
    fn from(fname: PathBuf) -> Result<Self> {
        trace!("state::from({:?}", fname);
        let data = fs::read_to_string(fname)?;
        let data: State = serde_json::from_str(&data)?;
        Ok(data)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_new() {
        let s = State::new();

        assert_eq!(0, s.last);
        assert!(s.queue.is_empty());
    }

    #[test]
    fn test_state_remove() {
        let mut s = State::new();

        s.queue.push_back(666);
        assert_eq!(1, s.queue.len());

        let s = s.remove_job(666);
        assert_eq!(0, s.last);
        dbg!(&s.queue);
        assert!(s.queue.is_empty());
    }
}
