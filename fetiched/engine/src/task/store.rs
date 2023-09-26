//! Special task that will store input in a tree of files, one every hour for now.
//!
//! 1. create a directory with the job ID
//! 2. store all data coming from the pipe in files every hour
//!
//! FIXME: make it configurable?
//!
//! This module is data-agnostic and does not care whether it is JSON, binary or a CSV.
//!

use std::fs;
use std::fs::{create_dir, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use chrono::{Datelike, Timelike, Utc};
use eyre::Result;
use tracing::trace;

use engine_macros::RunnableDerive;
use fetiche_sources::makepath;

use crate::{Runnable, IO};

/// Struct describing the data for the `Store` task.
///
/// We currently do not cache the open file for the current output, we might
/// do that in the future but the cost is 2 more syscalls but simplified code.
///
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
    /// Given a base directory in `path` create the tree if not present and store the full
    /// path as path/ID
    ///
    #[tracing::instrument]
    pub fn new(path: &str, id: usize) -> Self {
        trace!("store::new({})", path);

        // We want to have `path/current` pointing to `path/ID`
        //
        let id = format!("{id}");
        let base = path.clone();
        let path: PathBuf = makepath!(path, &id);

        // Base is PATH/ID/
        //
        create_dir(&path)
            .unwrap_or_else(|_| panic!("can not create {} in {}", id, path.to_string_lossy()));

        let curr: PathBuf = makepath!(&base, "current");
        if curr.exists() {
            fs::remove_file(&curr).expect("can not remove current");
        }

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&path, &curr).expect("can not symlink current");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&path, &curr).expect("can not symlink current");

        trace!("store::new({})", path.to_string_lossy());
        Store {
            io: IO::Consumer,
            path: Some(path),
        }
    }

    /// Store and rotate every hour for now.  We open/create and write every packet without
    /// trying to open first.  More syscalls but these are cheap.
    ///
    #[tracing::instrument]
    pub fn execute(&mut self, data: String, _stdout: Sender<String>) -> Result<()> {
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

        Ok(())
    }
}
