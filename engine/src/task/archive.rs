//! `Archive` is a task for the Engine.
//!
//! It takes a path or a job number and takes all files streamed so far and generate a complete
//! file in different format.

use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use eyre::Result;
use tracing::trace;

use crate::{Runnable, IO};
use fetiche_macros::RunnableDerive;
use fetiche_sources::Sources;

#[derive(Clone, Debug, RunnableDerive)]
pub struct Archive {
    io: IO,
    srcs: Arc<Sources>,
}

impl Archive {
    #[tracing::instrument(skip(srcs))]
    pub fn new(s: &str, srcs: Arc<Sources>) -> Self {
        Archive {
            io: IO::Consumer,
            srcs: srcs.clone(),
        }
    }

    #[tracing::instrument(skip(self))]
    fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        todo!()
    }
}
