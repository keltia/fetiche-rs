//! `Save` is a `Runnable` task as defined in the `engine`  crate.
//!
//! This is for saving data into a specific (or not) format like plain file (None) or Parquet.
//!

use std::fs::File;
use std::io::{BufReader, Seek};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use arrow2;
use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use arrow2::io::ndjson::read::FallibleStreamingIterator;
use arrow2::io::parquet::write::{
    transverse, CompressionOptions, Encoding, FileWriter, RowGroupIterator, Version, WriteOptions,
    ZstdLevel,
};
use eyre::Result;
use serde_arrow::arrow2::{serialize_into_arrays, serialize_into_fields};
use serde_arrow::schema::TracingOptions;
use serde_json::Deserializer;
use tap::Tap;
use tracing::{debug, info, trace};

use fetiche_formats::{Asd, Format};
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

                        let topts = TracingOptions::default()
                            .guess_dates(true)
                            .allow_null_fields(true);

                        let reader = BufReader::new(data.as_bytes());
                        let json = Deserializer::from_reader(reader).into_iter::<Asd>();

                        let data: Vec<Asd> = json.map(|e| e.unwrap().fix_tm().unwrap()).collect();

                        let fields = serialize_into_fields(&data, topts)?;
                        trace!("fields={:?}", fields);

                        let schema = Schema::from(fields.clone());
                        debug!("schema={:?}", schema);

                        let arrays = serialize_into_arrays(&fields, &data)?;
                        debug!("arrays={:?}", arrays);

                        trace!("{} records", arrays.len());
                        let _ = write_output(schema, arrays, p);
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
fn write_output(schema: Schema, data: Vec<Box<dyn Array>>, base: &str) -> Result<()> {
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(Some(ZstdLevel::default())),
        version: Version::V2,
        data_pagesize_limit: None,
    };

    debug!("data in={:?}", data);

    // Prepare output
    //
    let fname = format!("{}2.parquet", base);
    let file = File::create(&fname)?;

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

    info!("Done.");
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
