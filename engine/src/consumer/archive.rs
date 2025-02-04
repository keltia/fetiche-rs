//! `Archive` is a task for the Engine.
//!
//! It takes a path or a job number and takes all files streamed so far and generate a complete
//! file in different format.
//!
//! FIXME: incomplete

use eyre::Result;
use tokio::sync::mpsc::Sender;
use tracing::trace;

use crate::{Consumer, Fetch, IO, Producer, Runnable, Sources, Stats};
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
    #[tracing::instrument(skip(srcs))]
    pub fn new(s: &str) -> Self {
        trace!("Creating archive {s}");
        Archive { io: IO::Consumer }
    }

    #[tracing::instrument(skip(self, _stdout))]
    pub async fn execute(&mut self, _data: String, _stdout: Sender<String>) -> Result<()> {
        todo!()
    }
}
