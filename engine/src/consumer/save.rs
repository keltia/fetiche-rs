//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use eyre::Result;
use polars::prelude::*;
use tracing::{info, trace};

use fetiche_common::Container;
use fetiche_formats::Format;
use fetiche_macros::RunnableDerive;

use crate::{Consumer, Runnable, IO};

/// The Save task
///
#[derive(Clone, Debug, RunnableDerive, PartialEq)]
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

impl From<Save> for Consumer {
    fn from(f: Save) -> Self {
        Consumer::Save(f)
    }
}

impl Save {
    /// Initialise our environment
    ///
    #[tracing::instrument]
    pub fn new(name: &str, inp: Format, out: Container) -> Self {
        Save {
            io: IO::Consumer,
            name: name.to_owned(),
            path: None,
            inp,
            out,
            args: "".to_string(),
        }
    }

    /// Set the input path (for files) and infer the output container from the file extension.
    ///
    #[tracing::instrument(skip(self))]
    pub fn path(&mut self, name: &str) -> &mut Self {
        self.path = match name {
            "-" => None,
            _ => Some(name.to_string()),
        };
        let c = Container::from(name);
        self.out = c;
        self
    }

    /// The heart of the matter: save data
    ///
    #[tracing::instrument(skip(self, data))]
    pub async fn execute(&mut self, data: String, _stdout: Sender<String>) -> Result<()> {
        if self.path.is_none() {
            trace!("...into stdout");

            println!("{}", data);
        } else {
            let p = self.path.as_ref().unwrap();
            trace!("Writing into {}", p);

            match self.out {
                // There we handle the combination of input & output formats
                //
                Container::Parquet => {
                    trace!("from csv to parquet");

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
    use std::sync::mpsc::channel;

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

    #[test]
    fn test_write_with_args() {
        let mut t = Save::new("test_with_args", Format::None, Container::default());
        t.args = "{\"key\":\"value\"}".to_string();

        assert_eq!("test_with_args", t.name);
        assert_eq!("{\"key\":\"value\"}", t.args);
    }

    #[tokio::test]
    async fn test_save_to_parquet_with_nonexistent_path() {
        let mut t = Save::new("test_parquet_save", Format::Asd, Container::Parquet);
        t.path("/invalid/path/output.parquet");

        let (tx, _rx) = channel::<String>();
        let result = t.execute("dummy_data".to_string(), tx).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_save_output_raw_data() {
        let mut t = Save::new("test_raw_write", Format::None, Container::Raw);
        t.path("test_output.txt");

        let (tx, _rx) = channel::<String>();
        let result = t.execute("test_raw_data".to_string(), tx).await;

        assert!(result.is_ok());
        assert_eq!(
            fs::read_to_string("test_output.txt").unwrap(),
            "test_raw_data"
        );

        // Clean up
        fs::remove_file("test_output.txt").unwrap();
    }

    #[tokio::test]
    async fn test_save_stdout() {
        let mut t = Save::new("test_stdout", Format::None, Container::Raw);
        t.path("-");

        let (tx, _rx) = channel();
        let result = t.execute("output_to_stdout".to_string(), tx).await;

        assert!(result.is_ok());
        // Verifying stdout is out of scope, but it should print to console
    }
}
