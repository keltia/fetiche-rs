//! Module handling the conversions between different formats
//!
//! Currently supported:
//! - Input: Asd, Opensky
//! - Output: DronePoint
//!

use std::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use fetiche_formats::{prepare_csv, DronePoint, Format};
use fetiche_macros::RunnableDerive;

#[cfg(feature = "avionix")]
use fetiche_formats::avionix::CubeData;
#[cfg(feature = "senhive")]
use fetiche_formats::senhive::FusedData;
#[cfg(feature = "opensky")]
use fetiche_formats::StateList;

use crate::{Middle, Runnable, IO};

#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Convert {
    io: IO,
    pub from: Format,
    pub into: Format,
}

impl From<Convert> for Middle {
    fn from(t: Convert) -> Self {
        Middle::Convert(t.clone())
    }
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
    pub async fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("convert::execute");

        // Bow out early
        //
        let res = match self.into {
            // This one is always enabled.
            //
            Format::DronePoint => {
                let res: Vec<_> = match self.from {
                    #[cfg(feature = "avionix")]
                    Format::CubeData => {
                        trace!("cube_data:json to dronepoint: {}", data);

                        let r: Vec<CubeData> = serde_json::from_str(&data)?;
                        let r: Vec<_> = r.iter().map(DronePoint::from).collect();
                        r
                    }
                    #[cfg(feature = "senhive")]
                    Format::Senhive => {
                        trace!("senhive:json to dronepoint: {}", data);

                        let r: Vec<FusedData> = serde_json::from_str(&data)?;
                        let r: Vec<_> = r.iter().map(DronePoint::from).collect();
                        r
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
