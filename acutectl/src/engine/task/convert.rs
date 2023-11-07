//! Module handling the conversions between different formats
//!
//! Currently supported:
//! - Input: Asd, Opensky
//! - Output: Cat21
//!

use std::sync::mpsc::Sender;

use eyre::Result;
use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::schema::types::TypePtr;
use parquet::{
    file::{properties::WriterProperties, writer::SerializedFileWriter},
    record::RecordWriter,
};
use serde_json::json;
use tracing::trace;

use fetiche_formats::{prepare_csv, Asd, Cat21, Format, StateList};
use fetiche_macros::RunnableDerive;

use crate::{version, Runnable, IO};

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
    /// FIXME: only output Cat21 for now.
    ///
    #[tracing::instrument(skip(self))]
    pub fn execute(&mut self, data: String, stdout: Sender<String>) -> Result<()> {
        trace!("into::execute");

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
                    Format::Flightaware => {
                        trace!("flightaware:json to cat21: {}", data);

                        Cat21::from_flightaware(&data)?
                    }
                    _ => unimplemented!(),
                };
                prepare_csv(res, false)?
            }
            Format::Parquet => match self.from {
                Format::Asd => {
                    trace!("from asd to parquet");

                    let data: &[Asd] = serde_json::from_str(&data)?;

                    let mut res = String::new();
                    trace!("{} records", data.len());
                    let schema: TypePtr = data[0].schema()?;

                    let props = WriterProperties::builder()
                        .set_created_by(version())
                        .set_encoding(Encoding::PLAIN)
                        .set_compression(Compression::ZSTD(ZstdLevel::default()))
                        .build();

                    let mut writer = SerializedFileWriter::new(res, schema, props.into())?;
                    let mut row_group = writer.next_row_group()?;

                    trace!("Writing data.");
                    data.iter()
                        .for_each(|line| line.write_to_row_group(&mut row_group).unwrap());
                    trace!("Done.");
                    res.clone()
                }
                _ => unimplemented!(),
            },
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
