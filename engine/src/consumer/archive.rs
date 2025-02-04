//! `Archive` is a task for the Engine.
//!
//! It takes a path or a job number and takes all files streamed so far and generate a complete
//! file in different format.
//!
//! FIXME: incomplete

use std::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use crate::{Consumer, Runnable, IO};
use fetiche_macros::RunnableDerive;

#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Archive {
    io: IO,
}

impl From<Archive> for Consumer {
    fn from(f: Archive) -> Self {
        Consumer::Archive(f)
    }
}

impl Archive {
    #[tracing::instrument]
    pub fn new(s: &str) -> Self {
        trace!("Creating archive {s}");
        Archive { io: IO::Consumer }
    }

    #[tracing::instrument(skip(self, _stdout))]
    pub async fn execute(&mut self, _data: String, _stdout: Sender<String>) -> Result<()> {
        todo!()
    }
}
