//! Module handling the conversions between different formats
//!
//! Currently supported:
//! - Input: Avionix Cube data and Senhive fused data when enabled
//! - Output: DronePoint
//!

use tokio::sync::mpsc::Sender;

use eyre::Result;
use tracing::trace;

use fetiche_formats::Format;
#[cfg(any(feature = "avionix", feature = "senhive"))]
use fetiche_formats::{DronePoint, prepare_csv};
use fetiche_macros::RunnableDerive;

#[cfg(feature = "avionix")]
use fetiche_formats::avionix::CubeData;
#[cfg(feature = "senhive")]
use fetiche_formats::senhive::FusedData;

use crate::{IO, Middle, Runnable};

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

    /// Converts an enabled source format to CSV and sends it to the next stage.
    /// Unsupported format pairs return an error.
    #[tracing::instrument(skip(self, data, stdout))]
    pub async fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("convert::execute");

        let stage = self.clone();
        let res = tokio::task::spawn_blocking(move || stage.convert(data)).await??;
        Ok(stdout.send(res).await?)
    }

    /// Converts one configured JSON payload into DronePoint CSV.
    ///
    /// This synchronous helper runs on Tokio's blocking executor when called by
    /// [`Self::execute`]. Unsupported format pairs return an error.
    fn convert(&self, data: String) -> Result<String> {
        match (self.from, self.into, data) {
            #[cfg(feature = "avionix")]
            (Format::CubeData, Format::DronePoint, data) => {
                let rows: Vec<CubeData> = serde_json::from_str(&data)?;
                prepare_csv(rows.iter().map(DronePoint::from).collect(), false)
            }
            #[cfg(feature = "senhive")]
            (Format::Senhive, Format::DronePoint, data) => {
                let rows: Vec<FusedData> = serde_json::from_str(&data)?;
                prepare_csv(rows.iter().map(DronePoint::from).collect(), false)
            }
            (from, into, _) => Err(eyre::eyre!("unsupported conversion from {from} to {into}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unsupported format pair fails without sending data or panicking.
    #[tokio::test]
    async fn unsupported_conversion_returns_an_error() {
        let mut stage = Convert::new();
        stage.from(Format::None).into(Format::DronePoint);
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let error = stage.execute("[]".into(), tx).await.unwrap_err();
        assert!(error.to_string().contains("unsupported conversion"));
        assert!(rx.recv().await.is_none());
    }
}

impl Default for Convert {
    fn default() -> Self {
        Self::new()
    }
}
