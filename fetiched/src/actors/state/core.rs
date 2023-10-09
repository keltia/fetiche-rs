use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::trace;

/// We use a verion number for the state file to detect migrations and obsolete states.
///
pub const STATE_VERSION: usize = 1;

/// Register the state of the running system.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct State {
    /// Version
    pub version: usize,
    /// Timestamp
    pub tm: i64,
    /// Dirty bit
    pub dirty: bool,
    /// Hash table for each sub-system's state
    pub systems: HashMap<String, String>,
}

impl State {
    /// Create an clean and empty state
    ///
    pub fn new() -> Self {
        State {
            version: STATE_VERSION,
            tm: Utc::now().timestamp(),
            dirty: true,
            systems: HashMap::<String, String>::new(),
        }
    }

    /// Read our JSON file, any error results in the state being ignored and wiped out
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

        assert_eq!(STATE_VERSION, s.version);
        assert!(!s.systems.is_empty());
        assert!(s.dirty);
        assert!(s.systems.is_empty());
    }
}
