//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs;
use std::fs::OpenOptions;
use std::io::{BufReader, Seek};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use arrow2;
use arrow2::array::Array;
use arrow2::io::ndjson::read;
use arrow2::io::ndjson::read::FallibleStreamingIterator;
use arrow2::io::parquet::write::{CompressionOptions, Version, WriteOptions, ZstdLevel};
use eyre::Result;
use tap::Tap;
use tracing::{debug, info, trace};

use fetiche_formats::Format;
use fetiche_macros::RunnableDerive;

use crate::{Runnable, IO};

const BATCH: usize = 1024;

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
    pub out: Format,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Save {
    /// Initialise our environment
    ///
    #[tracing::instrument]
    pub fn new(name: &str, inp: Format, out: Format) -> Self {
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
        self.path = Some(name.to_string());
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
            info!("Writing into {}", p);

            match self.out {
                // There we handle the combination of input & output formats
                //
                Format::Parquet => match self.inp {
                    Format::Asd => {
                        trace!("from asd to parquet");

                        let mut reader = BufReader::new(data);

                        let dt = Arc::new(read::infer(&mut reader, None)?);
                        reader.rewind()?;
                        debug!("dt={:?}", dt);

                        let mut reader =
                            read::FileReader::new(reader, vec!["".to_string(); BATCH], None);
                        let mut arrays = vec![];

                        while let Some(rows) = reader.next()? {
                            let array = read::deserialize(rows, dt.into())?;
                            arrays.push(array);
                        }
                        debug!("arrays={:?}", arrays);

                        trace!("{} records", arrays.len());
                        let _ = write_output(&arrays, p);
                    }
                    _ => unimplemented!(),
                },
                _ => {
                    trace!("raw data");
                    fs::write(PathBuf::from(p), &data)?
                }
            };
        }
        Ok(())
    }
}

/// Write output from `Asd`  into proper `Parquet` file.
///
#[tracing::instrument(skip(data))]
fn write_output(data: &Vec<Box<dyn Array>>, out: &str) -> Result<()> {
    // Prepare output
    //
    let fh = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out)?;

    let props = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(Some(ZstdLevel::default())),
        version: Version::V2,
        data_pagesize_limit: None,
    };

    let schema = data.as_slice().schema()?;

    info!("Writing in {}", out);
    let mut writer = SerializedFileWriter::new(fh, schema.clone(), props.into())?;
    let mut row_group = writer.next_row_group()?;

    trace!("Writing data.");
    data.as_slice()
        .tap(|&e| trace!("e={:?}", e))
        .write_to_row_group(&mut row_group)?;
    let m = row_group.close()?;
    trace!("{} records written.", m.num_rows());
    writer.close()?;

    trace!("Done.");
    Ok(())
}

impl Default for Save {
    fn default() -> Self {
        Save::new("default", Format::None, Format::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_new() {
        let t = Save::new("foo");

        assert_eq!("foo", t.name);
        assert!(t.path.is_none());
    }

    #[test]
    fn test_write_stdout() {
        let mut t = Save::new("foo");
        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        assert_eq!(PathBuf::from("/nonexistent"), t.path.unwrap());
    }

    #[test]
    fn test_write_file() {
        let mut t = Save::new("foo");
        t.path("../Cargo.toml");

        assert_eq!("foo", t.name);
        assert_eq!(PathBuf::from("../Cargo.toml"), t.path.unwrap());
    }
}
