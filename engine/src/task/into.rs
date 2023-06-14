//! Module handling the conversions between different formats
//!

use std::sync::mpsc::Sender;

use anyhow::Result;

use engine_macros::RunnableDerive;
use fetiche_formats::Format;

use crate::Runnable;

#[derive(Clone, Debug, RunnableDerive)]
pub struct Into {
    pub from: Format,
    pub into: Format,
}

impl Into {
    pub fn new() -> Self {
        Self { from: Format::None, into: Format::None }
    }

    pub fn from(&mut self, frm: Format) -> &mut Self {
        self.from = frm;
        self
    }

    pub fn into(&mut self, frm: Format) -> &mut Self {
        self.into = frm;
        self
    }

    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        Ok(stdout.send(data)?)
    }
}
