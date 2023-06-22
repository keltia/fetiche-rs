//! This is a task module that act like the UNIX command of the same name:
//! copy whatever it receive into a file and pass the data down the pipe
//! unchanged
//!

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::trace;

use engine_macros::RunnableDerive;

use crate::{Runnable, IO};

#[derive(Clone, Debug, RunnableDerive)]
pub struct Tee {
    io: IO,
    pub fh: Arc<Mutex<File>>,
}

impl Tee {
    #[inline]
    pub fn into(p: &str) -> Self {
        let path = PathBuf::from(p);
        Tee {
            io: IO::InOut,
            fh: Arc::new(Mutex::new(File::create(path).unwrap())),
        }
    }

    /// This is the main task.  Every data packet we receive will be written in the designed
    /// file then passed down.
    ///
    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("tee::execute");
        let mut fh = self.fh.lock().unwrap();
        write!(fh, "{data}")?;
        Ok(stdout.send(data)?)
    }
}
