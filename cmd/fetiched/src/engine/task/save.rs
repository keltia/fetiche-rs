//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs;
use std::fs::OpenOptions;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use eyre::Result;
use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::schema::types::TypePtr;
use parquet::{
    file::{properties::WriterProperties, writer::SerializedFileWriter},
    record::RecordWriter,
};
use tracing::{info, trace};

use fetiche_formats::{Asd, Format};
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
    pub out: Format,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Save {
    /// Initialize our environment
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

            Ok(write!(stdout(), "{}", data)?)
        } else {
            let p = self.path.as_ref().unwrap();
            info!("Writing into {}", p);

            match self.out {
                // There we handle the combination of input & output formats
                //
                Format::Parquet => match self.inp {
                    Format::Asd => {
                        trace!("from asd to parquet");

                        let data: Vec<Asd> = serde_json::from_str(&data)?;

                        trace!("{} records", data.len());
                        let schema = data.as_slice().schema()?;

                        let _ = write_output(schema, &data, p);
                    }
                    _ => unimplemented!(),
                },
                _ => {
                    trace!("raw data");
                    fs::write(PathBuf::from(p), &data)?
                }
            };
            Ok(())
        }
    }
}

/// Write output from `Asd`  into proper `Parquet` file.
///
#[tracing::instrument(skip(data, schema))]
fn write_output(schema: TypePtr, data: &Vec<Asd>, out: &str) -> Result<()> {
    // Prepare output
    //
    let fh = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(out)?;

    let props = WriterProperties::builder()
        .set_created_by("fetiche".to_string())
        .set_encoding(Encoding::PLAIN)
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    info!("Writing in {}", out);
    let mut writer = SerializedFileWriter::new(fh, schema, props.into())?;
    let mut row_group = writer.next_row_group()?;

    trace!("Writing data.");
    let _ = data.as_slice().write_to_row_group(&mut row_group)?;
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
