//! Keeping state in Fetiche
//!

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::trace;

use crate::Engine;

pub const STATE_FILE: &str = "state";

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
    pub fn from(fname: PathBuf) -> Result<Self> {
        trace!("state::from");
        let data = fs::read_to_string(fname)?;
        let data: State = serde_json::from_str(&data)?;
        Ok(data)
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
    pub fn sync(&self) -> Result<()> {
        trace!("engine::sync");
        let mut data = self.state.write().unwrap();
        *data = State {
            tm: Utc::now().timestamp(),
            last: *data.queue.back().unwrap_or_else(|| &1),
            queue: data.queue.clone(),
        };
        let data = json!(*data).to_string();
        Ok(fs::write(self.state_file(), &data)?)
    }
}
