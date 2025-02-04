use std::sync::mpsc::Sender;

use eyre::Result;

use crate::{Runnable, IO};

use fetiche_macros::RunnableDerive;

#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Dummy {
    io: IO,
}

impl Dummy {
    pub fn new() -> Self {
        Self { io: IO::Producer }
    }

    #[tracing::instrument(skip(self))]
    pub fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<()> {
        stdout.send("DUMMY".to_string())?;
        Ok(())
    }
}

impl Default for Dummy {
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
    }
}
