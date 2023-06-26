//! Special task that will
//!
//! 1. create a directory with the job ID
//! 2. store all data coming from the pipe in files every hour
//!

use std::fs::{create_dir, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use anyhow::Result;
use chrono::{Datelike, Utc};
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
        create_dir(&path).expect(&format!(
            "can not create {} in {}",
            id,
            path.to_string_lossy()
        ));

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
        let (year, month, day) = (tm.year(), tm.month(), tm.day());
        let fname = format!("{}{}{}-000000", year, month, day);

        // Full path is BASE/ID/FNAME
        //
        let base: PathBuf = [self.path.clone().unwrap(), PathBuf::from(&self.id)]
            .iter()
            .collect();
        let path: PathBuf = [base, PathBuf::from(fname)].iter().collect();

        // Create if not present
        //
        if !path.exists() {
            File::create(&path)?;
        }

        // Append to it
        //
        let mut fh = OpenOptions::new().write(true).append(true).open(&path)?;
        write!(fh, "{}", data)?;

        Ok(stdout.send("w".to_string())?)
    }
}
