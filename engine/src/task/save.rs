//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

#[cfg(feature = "datafusion")]
use datafusion::{
    config::TableParquetOptions,
    dataframe::DataFrameWriteOptions,
    prelude::{CsvReadOptions, SessionContext},
};
#[cfg(feature = "polars")]
use polars::prelude::*;

use eyre::Result;
use tempfile::Builder;
use tokio::runtime::Runtime;
use tracing::{info, trace};

use fetiche_common::Container;
use fetiche_formats::Format;
use fetiche_macros::RunnableDerive;

use crate::{EngineStatus, Runnable, IO};

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
                            write_parquet(&fname, p).await.unwrap();
                        });
                    }
                    _ => return Err(EngineStatus::OnlyAsdToParquet.into()),
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

#[cfg(feature = "datafusion")]
/// Write parquet through datafusion.
///
#[tracing::instrument]
async fn write_parquet(from: &str, to: &str) -> Result<()> {
    let ctx = SessionContext::new();
    let df = ctx
        .read_csv(from, CsvReadOptions::default().has_header(true))
        .await?;
    let dfopts = DataFrameWriteOptions::default().with_single_file_output(true);

    let mut options = TableParquetOptions::default();
    options.global.created_by = "acutectl/save".to_string();
    options.global.writer_version = "2.0".to_string();
    options.global.encoding = Some("plain".to_string());
    options.global.statistics_enabled = Some("page".to_string());
    options.global.compression = Some("zstd(8)".to_string());

    let _ = df.write_parquet(to, dfopts, Some(options)).await?;
    Ok(())
}

#[cfg(feature = "polars")]
/// Write parquet through Polars
///
#[tracing::instrument]
async fn write_parquet(from: &str, to: &str) -> Result<()> {
    // nh = no header line (default = false which means has header line).
    //
    let header = true;

    let opts = CsvParseOptions::default().with_try_parse_dates(true);
    let mut df = CsvReadOptions::default()
        .with_has_header(header)
        .with_parse_options(opts)
        .try_into_reader_with_file_path(Some(from.into()))?
        .finish()?;

    let mut file = fs::File::create(to)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;
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
