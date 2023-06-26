//! Special task that will
//!
//! 1. create a directory with the job ID
//! 2. store all data coming from the pipe in files every hour
//!

use std::fs::{create_dir, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use anyhow::Result;
use chrono::{Datelike, Timelike, Utc};
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_sources::makepath;

use crate::{Runnable, IO};

#[derive(Clone, Debug, RunnableDerive)]
pub struct Store {
    /// IO Capability
    io: IO,
    /// Our storage directory
    path: Option<PathBuf>,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            io: IO::Consumer,
            path: None,
        }
    }
}

impl Store {
    pub fn new(path: &str, id: &str) -> Self {
        let path: PathBuf = makepath!(path, id);

        // Base is PATH/ID/
        //
        create_dir(&path)
            .unwrap_or_else(|_| panic!("can not create {} in {}", id, path.to_string_lossy()));

        trace!("store::new({})", path.to_string_lossy());
        Store {
            io: IO::Consumer,
            path: Some(path),
        }
    }

    /// Store is like
    ///
    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("store::execute");

        let tm = Utc::now();

        // Extract parts to get to a filename
        //
        // Filename format is YYYYMMDD-HH0000
        //
        let (year, month, day, hour) = (tm.year(), tm.month(), tm.day(), tm.hour());
        let fname = format!("{}{:02}{:02}-{:02}0000", year, month, day, hour);

        // Full path is BASE/ID/FNAME
        //
        let path: PathBuf = [self.path.clone().unwrap(), PathBuf::from(fname)]
            .iter()
            .collect();
        trace!("fname={}", path.to_string_lossy());

        // Append to it (and create if not yet present)
        //
        let mut fh = OpenOptions::new().create(true).append(true).open(&path)?;
        write!(fh, "{}", data)?;

        Ok(stdout.send("w".to_string())?)
    }
}
