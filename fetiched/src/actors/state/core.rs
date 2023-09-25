use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::trace;

/// Register the state of the running `Engine`.
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
}
