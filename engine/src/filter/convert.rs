//! Module handling the conversions between different formats
//!
//! Currently supported:
//! - Input: Asd, Opensky
//! - Output: Cat21
//!

use std::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use fetiche_formats::{prepare_csv, DronePoint, Format};
use fetiche_macros::RunnableDerive;

#[cfg(feature = "asterix")]
use fetiche_formats::Cat21;
#[cfg(feature = "opensky")]
use fetiche_formats::StateList;
#[cfg(feature = "avionix")]
use fetiche_formats::avionix::CubeData;
#[cfg(feature = "senhive")]
use fetiche_formats::senhive::FusedData;

use crate::{IO, Middle, Runnable, Tee};

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
            #[cfg(feature = "asterix")]
            Format::Cat21 => {
                let res: Vec<_> = match self.from {
                    #[cfg(feature = "opensky")]
                    Format::Opensky => {
                        trace!("opensky:json to cat21: {}", data);

                        let data: StateList = serde_json::from_str(&data)?;
                        trace!("data={:?}", data);
                        let data = json!(&data.states).to_string();
                        trace!("data={}", data);
                        Cat21::from_opensky(&data)?
                    }
                    #[cfg(feature = "asd")]
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
            // This one is always enabled.
            //
            Format::DronePoint => {
                let res: Vec<_> = match self.from {
                    #[cfg(feature = "avionix")]
                    Format::CubeData => {
                        trace!("cube_data:json to dronepoint: {}", data);

                        let r: Vec<CubeData> = serde_json::from_str(&data)?;
                        let r: Vec<_> = r.iter().map(|e| DronePoint::from(e)).collect();
                        r
                    }
                    #[cfg(feature = "senhive")]
                    Format::Senhive => {
                        trace!("senhive:json to dronepoint: {}", data);

                        let r: Vec<FusedData> = serde_json::from_str(&data)?;
                        let r: Vec<_> = r.iter().map(|e| DronePoint::from(e)).collect();
                        r
                    }
                    _ => unimplemented!(),
                };
                prepare_csv(res, false)?
            }
            _ => unimplemented!(),
        };

        Ok(stdout.send(res).await?)
    }
}

impl Default for Convert {
    fn default() -> Self {
        Self::new()
    }
}
