//! Module handling the conversions between different formats
//!
//! Currently supported:
//! - Input: Asd, Opensky
//! - Output: Cat21
//!

use std::sync::mpsc::Sender;

use eyre::Result;
use serde_json::json;
use tracing::trace;

use fetiche_formats::{prepare_csv, Cat21, Format, StateList};
use fetiche_macros::RunnableDerive;

use crate::{Runnable, IO};

#[derive(Clone, Debug, RunnableDerive)]
pub struct Convert {
    io: IO,
    pub from: Format,
    pub into: Format,
}

impl Convert {
    #[inline]
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            io: IO::Filter,
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
    #[tracing::instrument(skip(self))]
    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("convert::execute");

        // Bow out early
        //
        let res = match self.into {
            Format::Cat21 => {
                let res: Vec<_> = match self.from {
                    Format::Opensky => {
                        trace!("opensky:json to cat21: {}", data);

                        let data: StateList = serde_json::from_str(&data)?;
                        trace!("data={:?}", data);
                        let data = json!(&data.states).to_string();
                        trace!("data={}", data);
                        Cat21::from_opensky(&data)?
                    }
                    Format::Asd => {
                        trace!("asd:json to cat21: {}", data);

                        Cat21::from_asd(&data)?
                    }
                    #[cfg(feature = "flightaware")]
                    Format::Flightaware => {
                        trace!("flightaware:json to cat21: {}", data);

                        Cat21::from_flightaware(&data)?
                    }
                    _ => unimplemented!(),
                };
                prepare_csv(res, false)?
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
