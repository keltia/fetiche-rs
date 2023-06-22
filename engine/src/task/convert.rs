//! Module handling the conversions between different formats
//!
//! Currently supported:
//! - Input: Aeroscope, Asd, Opensky
//! - Output: DronePoint, Cat21, Cat129*
//!

use std::sync::mpsc::Sender;

use anyhow::Result;
use log::trace;

use engine_macros::RunnableDerive;
use fetiche_formats::{prepare_csv, Asd, Cat21, Format, StateList};

use crate::{Runnable, IO};

pub trait ConvertInto {
    fn convert(&self, into: Format) -> String;
}

#[derive(Clone, Debug, RunnableDerive)]
pub struct Convert {
    io: IO,
    pub from: Format,
    pub into: Format,
}

impl Convert {
    #[inline]
    pub fn new() -> Self {
        Self {
            io: IO::InOut,
            from: Format::None,
            into: Format::None,
        }
    }

    #[inline]
    pub fn from(&mut self, frm: Format) -> &mut Self {
        self.from = frm;
        self
    }

    #[inline]
    pub fn into(&mut self, frm: Format) -> &mut Self {
        self.into = frm;
        self
    }

    /// This is the task here, converting between format from the previous stage
    /// of the pipeline and send it down to the next stage.
    ///
    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("into::execute");

        let res = match self.from {
            Format::Opensky => {
                let sl: StateList = serde_json::from_str(&data).unwrap();
                let r = sl.to_cat21();

                let res = prepare_csv(r, false).unwrap();
                res
            }
            Format::Asd => {
                trace!("asd:json to cat21: {}", data);

                let r = Cat21::from_asd(&data)?;
                let res = prepare_csv(r, true).unwrap();
                res
            }
            _ => unimplemented!(),
        };
        Ok(stdout.send(res)?)
    }
}

impl Default for Convert {
    fn default() -> Self {
        Self::new()
    }
}
