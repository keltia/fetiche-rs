//! Special task that will
//!
//! 1. create a directory with the job ID
//! 2. store all data coming from the pipe in files every hour
//!

use std::fs::create_dir;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use anyhow::Result;
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_sources::makepath;

use crate::{Runnable, IO};

#[derive(Clone, Debug, RunnableDerive)]
pub struct Store {
    /// IO Capability
    io: IO,
    /// Job ID
    id: String,
    /// Our storage directory
    path: Option<PathBuf>,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            io: IO::In,
            id: "".to_string(),
            path: None,
        }
    }
}

impl Store {
    pub fn new(path: &str, id: &str) -> Self {
        let path = makepath!(path, id);

        create_dir(path).expect(&format!("can not create {} in {}", id, path));

        Store {
            io: IO::In,
            id: id.to_owned(),
            path: Some(path.clone()),
        }
    }

    /// Store is like
    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(())
    }
}
