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
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use chrono::{Datelike, Timelike, Utc};
use eyre::Result;
use tracing::{error, trace};

use fetiche_macros::RunnableDerive;

use crate::{EngineStatus, Runnable, IO};

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
    path: PathBuf,
}

impl Default for Store {
    fn default() -> Self {
        Store {
            io: IO::Consumer,
            path: PathBuf::from(""),
        }
    }
}

impl Store {
    /// Given a base directory in `path` create the tree if not present and store the full
    /// path as path/ID
    ///
    #[tracing::instrument]
    pub fn new(path: &str, id: usize) -> Result<Self> {
        trace!("store::new");

        // Ensure path is defined.
        //
        if path.is_empty() {
            error!("Store: path can not be empty");
            return Err(EngineStatus::NoPathDefined.into());
        }

        // We want to have `path/current` pointing to `path/ID`
        //
        let base = PathBuf::from(path);
        let path = base.join(id.to_string());
        trace!("Store path is {}", path.to_string_lossy().to_string());

        // Base MUST be writable so we create BASE/ID
        //
        if !path.exists() {
            trace!("Store: creating {}", path.to_string_lossy().to_string());

            if let Err(e) = fs::create_dir_all(&path) {
                let path = path.to_string_lossy().to_string();
                error!("Store: can not create {}: {}", path, e.to_string());
            }
        }

        let curr = base.join("current");
        if curr.exists() {
            if let Err(e) = fs::remove_file(&curr) {
                let curr = curr.to_string_lossy().to_string();

                error!("Store: can not remove symlink {}: {}", curr, e.to_string());
                return Err(EngineStatus::RemoveLink(curr).into());
            }
        }

        #[cfg(windows)]
        if let Err(e) = std::os::windows::fs::symlink_dir(&path, &curr) {
            let path = path.to_string_lossy().to_string();
            let curr = curr.to_string_lossy().to_string();

            error!(
                "Store: can not create symlink to {} as {}: {}",
                path,
                curr,
                e.to_string()
            );
            return Err(EngineStatus::CreateLink(path, curr).into());
        }

        #[cfg(unix)]
        if let Err(e) = std::os::unix::fs::symlink(&path, &curr) {
            let path = path.to_string_lossy().to_string();
            let curr = curr.to_string_lossy().to_string();

            error!(
                "Store: can not create symlink to {} as {}: {}",
                path,
                curr,
                e.to_string()
            );
            return Err(EngineStatus::CreateLink(path, curr));
        }

        Ok(Store {
            io: IO::Consumer,
            path,
        })
    }

    /// Store and rotate every hour for now.  We open/create and write every packet without
    /// trying to open first.  More syscalls but these are cheap.
    ///
    #[tracing::instrument(skip(self, _stdout))]
    pub fn execute(&mut self, data: String, _stdout: Sender<String>) -> Result<()> {
        trace!("store::execute");

        let tm = Utc::now();

        // Extract parts to create a filename
        //
        // Filename format is YYYYMMDD-HH0000
        //
        let (year, month, day, hour) = (tm.year(), tm.month(), tm.day(), tm.hour());
        let fname = format!("{}{:02}{:02}-{:02}0000", year, month, day, hour);

        // Full path is BASE/ID/FNAME
        //
        let base = self.path.clone();
        let fname = base.join(fname);

        trace!("final name={}", fname.to_string_lossy().to_string());

        // Append to it (and create if not yet present)
        //
        let mut fh = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(fname)?;
        write!(fh, "{}", data)?;
        Ok(())
    }
}
