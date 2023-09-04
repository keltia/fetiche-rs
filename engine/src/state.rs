//! Keeping state in Fetiche
//!

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::trace;

use crate::{Engine, STATE_FILE};

/// Register the state of the running `Engine`.
///
/// NOTE: At the moment, the is not `fetiched` daemon, it is all in a single
/// binary.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct State {
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
    pub fn from(fname: PathBuf) -> Result<Self> {
        trace!("state::from({:?}", fname);
        let data = fs::read_to_string(fname)?;
        let data: State = serde_json::from_str(&data)?;
        Ok(data)
    }

    /// Perform a binary search on the job queue (job id are always incrementing) and remove said
    /// job (done or cancelled, etc.).
    ///
    #[tracing::instrument]
    pub fn remove_job(&mut self, id: usize) -> &mut Self {
        trace!("state::remove_job({})", id);
        if let Ok(index) = self.queue.binary_search(&id) {
            self.queue.remove(index);
        }
        self
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Returns the path of the default state file in basedir
    ///
    pub fn state_file(&self) -> PathBuf {
        self.home.join(STATE_FILE)
    }

    /// Sync all state into a file
    ///
    #[tracing::instrument]
    pub fn sync(&self) -> Result<()> {
        trace!("engine::sync");
        let mut data = self.state.write().unwrap();
        *data = State {
            tm: Utc::now().timestamp(),
            last: *data.queue.back().unwrap_or(&1),
            queue: data.queue.clone(),
        };
        let data = json!(*data).to_string();
        Ok(fs::write(self.state_file(), data)?)
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
