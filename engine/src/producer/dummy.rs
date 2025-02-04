use std::sync::mpsc::Sender;

use eyre::Result;

use crate::{Runnable, Stats, IO};

use fetiche_macros::RunnableDerive;

#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Dummy {
    io: IO,
    stats: Stats,
}

impl Dummy {
    pub fn new() -> Self {
        Self { io: IO::Producer, stats: Stats::default() }
    }

    #[tracing::instrument(skip(self))]
    pub async fn execute(&mut self, _data: String, stdout: Sender<String>) -> Result<Stats> {
        stdout.send("DUMMY".to_string())?;
        Ok(Stats::default())
    }
}

impl Default for Dummy {
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
    }
}
