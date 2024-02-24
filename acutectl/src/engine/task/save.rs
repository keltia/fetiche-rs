//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::parquet::{
    basic::{Compression, Encoding, ZstdLevel},
    file::properties::{EnabledStatistics, WriterProperties},
};
use datafusion::prelude::{CsvReadOptions, SessionContext};

use eyre::{eyre, Result};
use tempfile::Builder;
use tokio::runtime::Runtime;
use tracing::{info, trace};

use fetiche_formats::{Container, Format};
use fetiche_macros::RunnableDerive;

use crate::{Runnable, IO};

/// The Save task
///
#[derive(Clone, Debug, RunnableDerive)]
pub struct Save {
    /// I/O capabilities
    io: IO,
    /// name for the task
    pub name: String,
    /// File path
    pub path: Option<String>,
    /// Input file format
    pub inp: Format,
    /// Output file format
    pub out: Container,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Save {
    /// Initialise our environment
    ///
    #[tracing::instrument]
    pub fn new(name: &str, inp: Format, out: Container) -> Self {
        trace!("New Save {}", name);
        Save {
            io: IO::Consumer,
            name: name.to_owned(),
            path: None,
            inp,
            out,
            args: "".to_string(),
        }
    }

    /// Set the input path (for files)
    ///
    pub fn path(&mut self, name: &str) -> &mut Self {
        trace!("Add path: {}", name);
        self.path = match name {
            "-" => None,
            _ => Some(name.to_string()),
        };
        self
    }

    /// The heart of the matter: save data
    ///
    #[tracing::instrument(skip(data))]
    pub fn execute(&mut self, data: String, _stdout: Sender<String>) -> Result<()> {
        trace!("Save::execute()");

        if self.path.is_none() {
            trace!("...into stdout");

            println!("{}", data);
        } else {
            let p = self.path.as_ref().unwrap();
            trace!("Writing into {}", p);

            match self.out {
                // There we handle the combination of input & output formats
                //
                Container::Parquet => match self.inp {
                    Format::Asd => {
                        trace!("from asd(csv) to parquet");

                        // Write into temporary file.
                        //
                        let mut tmpf = Builder::new().suffix(".csv").tempfile()?;
                        let _ = tmpf.write(data.as_bytes())?;

                        let fname = tmpf.path().to_string_lossy().to_string();
                        info!("fname={}, p={}", fname, p);

                        // Create tokio runtime
                        //
                        let rt = Runtime::new()?;

                        rt.block_on(async {
                            let _ = write_parquet(&fname, &p).await.unwrap();
                        });
                    }
                    _ => return Err(eyre!("Error: only Asd is supported as input.")),
                },
                _ => {
                    trace!("raw data");
                    fs::write(PathBuf::from(p), &data)?
                }
            }
        }
        Ok(())
    }
}

/// Write parquet through datafusion.
///
#[tracing::instrument]
async fn write_parquet(from: &str, to: &str) -> Result<()> {
    let ctx = SessionContext::new();
    let df = ctx.read_csv(from, CsvReadOptions::default()).await?;
    let dfopts = DataFrameWriteOptions::default().with_single_file_output(true);

    let props = WriterProperties::builder()
        .set_created_by("acutectl/save".to_string())
        .set_encoding(Encoding::PLAIN)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_compression(Compression::ZSTD(ZstdLevel::try_new(8).unwrap()))
        .build();
    let _ = df.write_parquet(&to, dfopts, Some(props)).await?;
    Ok(())
}

impl Default for Save {
    fn default() -> Self {
        Save::new("default", Format::None, Container::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_new() {
        let t = Save::new("foo", Format::None, Container::default());

        assert_eq!("foo", t.name);
        assert!(t.path.is_none());
    }

    #[test]
    fn test_write_stdout() {
        let mut t = Save::new("foo", Format::None, Container::default());
        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        assert_eq!("/nonexistent", t.path.unwrap());
    }

    #[test]
    fn test_write_file() {
        let mut t = Save::new("foo", Format::None, Container::default());
        t.path("../Cargo.toml");

        assert_eq!("foo", t.name);
        assert_eq!("../Cargo.toml", t.path.unwrap());
    }
}
