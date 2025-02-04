//! `Archive` is a task for the Engine.
//!
//! It takes a path or a job number and takes all files streamed so far and generate a complete
//! file in different format.
//!
//! FIXME: incomplete

use eyre::Result;
use std::sync::Arc;

use tokio::sync::mpsc::Sender;
use tracing::trace;

use crate::{Runnable, Sources, Stats, IO};
use fetiche_macros::RunnableDerive;

#[derive(Clone, Debug, RunnableDerive)]
pub struct Archive {
    io: IO,
    srcs: Arc<Sources>,
}

impl Archive {
    #[tracing::instrument(skip(srcs))]
    pub fn new(s: &str, srcs: Arc<Sources>) -> Self {
        trace!("Creating archive {s}");
        Archive {
            io: IO::Consumer,
            srcs: srcs.clone(),
        }
    }

    #[tracing::instrument(skip(self, _stdout))]
    pub async fn execute(&mut self, _data: String, _stdout: Sender<String>) -> Result<()> {
        todo!()
    }
}
