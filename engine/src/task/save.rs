//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use polars::prelude::*;

use eyre::Result;
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
    #[tracing::instrument(skip(self))]
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
    #[tracing::instrument(skip(self, data))]
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

                        let cur = Cursor::new(&data);
                        let opts = CsvParseOptions::default().with_try_parse_dates(false);
                        let mut df = CsvReadOptions::default()
                            .with_has_header(true)
                            .with_parse_options(opts)
                            .into_reader_with_file_handle(cur)
                            .finish()?;

                        info!("writing {}", p);
                        let mut file = fs::File::create(p)?;

                        ParquetWriter::new(&mut file).finish(&mut df)?;
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
