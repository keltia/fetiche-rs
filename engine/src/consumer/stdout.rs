use std::sync::mpsc::Sender;

use eyre::Result;

use fetiche_macros::RunnableDerive;

use crate::{Runnable, IO};

#[derive(Clone, Debug, PartialEq, RunnableDerive)]
pub struct Stdout {
    io: IO,
}

impl Stdout {
    #[tracing::instrument]
    pub fn new() -> Self {
        Self { io: IO::Consumer }
    }

    #[tracing::instrument(skip(self, data, _out))]
    pub fn execute(&mut self, data: String, _out: Sender<String>) -> Result<()> {
        println!("{}", data);
        Ok(())
    }
}

impl Default for Stdout {
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
    }
}


