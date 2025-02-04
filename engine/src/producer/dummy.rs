use std::sync::mpsc::Sender;

use eyre::Result;

use crate::{Runnable, IO};

use fetiche_macros::RunnableDerive;

#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Dummy {
    pub io: IO,
}

impl Dummy {
    pub fn new() -> Self {
        Dummy {
            io: IO::Producer
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn execute(&self, _data: String, stdout: Sender<String>, _args: String) -> Result<()> {
        stdout.send("DUMMY".to_string())?;
        Ok(())
    }
}

impl Default for Dummy {
    fn default() -> Self {
        Self::new()
    }
}
