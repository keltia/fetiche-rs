//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use arrow2;
use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use arrow2::io::parquet::write::{
    transverse, CompressionOptions, Encoding, FileWriter, RowGroupIterator, Version, WriteOptions,
    ZstdLevel,
};
use eyre::Result;
use serde_arrow::schema::{SerdeArrowSchema, TracingOptions};
use serde_json::Deserializer;
use tracing::{debug, info, trace};

use fetiche_formats::{Asd, Format, Write};
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
    pub out: Write,
    /// Optional arguments (usually json-encoded string)
    pub args: String,
}

impl Save {
    /// Initialise our environment
    ///
    #[tracing::instrument]
    pub fn new(name: &str, inp: Format, out: Write) -> Self {
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
            trace!("Writing into {}", p);

            match self.out {
                // There we handle the combination of input & output formats
                //
                Write::Parquet => match self.inp {
                    Format::Asd => {
                        trace!("from asd to parquet");

                        let topts = TracingOptions::default()
                            .guess_dates(true)
                            .allow_null_fields(true);

                        let reader = BufReader::new(data.as_bytes());
                        let json = Deserializer::from_reader(reader).into_iter::<Asd>();

                        let data: Vec<_> = json
                            .map(|e| e.unwrap().fix_tm().unwrap())
                            .collect::<Vec<_>>();

                        let data = data.as_slice();
                        let fields =
                            SerdeArrowSchema::from_samples(&data, topts)?.to_arrow2_fields()?;
                        trace!("fields={:?}", fields);

                        let schema = Schema::from(fields.clone());
                        debug!("schema={:?}", schema);

                        let schema = Schema::from(fields.clone());
                        debug!("schema={:?}", schema);

                        let arrays = serde_arrow::to_arrow2(&fields, &data)?;
                        trace!("{} records", arrays.len());

                        let _ = write_parquet(schema, arrays, p);
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
#[tracing::instrument(skip(schema, data))]
fn write_parquet(schema: Schema, data: Vec<Box<dyn Array>>, base: &str) -> Result<()> {
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(Some(ZstdLevel::try_new(8)?)),
        version: Version::V2,
        data_pagesize_limit: None,
    };

    debug!("data in={:?}", data);

    // Prepare output
    //
    let file = File::create(base)?;

    let iter = vec![Ok(Chunk::new(data))];
    debug!("iter={:?}", iter);

    let encodings = schema
        .fields
        .iter()
        .map(|f| transverse(&f.data_type, |_| Encoding::Plain))
        .collect();

    let row_groups = RowGroupIterator::try_new(iter.into_iter(), &schema, options, encodings)?;
    let mut writer = FileWriter::try_new(file, schema, options)?;

    for group in row_groups {
        writer.write(group?)?;
    }

    let size = writer.end(None)?;
    trace!("{} bytes written.", size);

    info!("Writing, done.");
    Ok(())
}

impl Default for Save {
    fn default() -> Self {
        Save::new("default", Format::None, Write::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_new() {
        let t = Save::new("foo", Format::None, Write::default());

        assert_eq!("foo", t.name);
        assert!(t.path.is_none());
    }

    #[test]
    fn test_write_stdout() {
        let mut t = Save::new("foo", Format::None, Write::default());
        t.path("/nonexistent");

        assert_eq!("foo", t.name);
        assert_eq!("/nonexistent", t.path.unwrap());
    }

    #[test]
    fn test_write_file() {
        let mut t = Save::new("foo", Format::None, Write::default());
        t.path("../Cargo.toml");

        assert_eq!("foo", t.name);
        assert_eq!("../Cargo.toml", t.path.unwrap());
    }
}
