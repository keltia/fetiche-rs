//! Special task that will store input in a database
//!
//! NOTE: This module is NOT data-agnostic, you have to specify the input format
//!       and the database on initialisation.
//!

use std::sync::mpsc::Sender;

use eyre::Result;

use fetiche_formats::Format;
use fetiche_macros::RunnableDerive;

use crate::IO;

#[derive(Clone, Debug, RunnableDerive)]
pub struct Record {
    /// IO Capability
    io: IO,
    /// Input format
    fmt: Format,
    /// DB name
    db: Option<String>,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            io: IO::Consumer,
            fmt: Format::Cat21,
            db: None,
        }
    }
}

impl Record {
    pub fn new(fmt: Format, db: String) -> Self {
        Record {
            fmt,
            db: Some(db),
            ..Self::default()
        }
    }

    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(())
    }
}
