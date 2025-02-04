//! Read some data as json and write it into a parquet file
//!

use std::fs::File;
use std::num::NonZeroUsize;
use std::path::Path;

use eyre::Result;
use polars::prelude::{JsonFormat, JsonReader, ParquetWriter, SerReader};

/// Reads JSON data from a file, processes it into a DataFrame,
/// and writes it out as a Parquet file.
///
/// # Arguments
///
/// * `base` - The base filename (without extension) to use for input and output.
///            The function expects a file with `.json` extension for input
///            and will write a file with `.parquet` extension for output.
///
/// # Errors
///
/// Returns an error if:
/// - The input JSON file cannot be found or opened.
/// - There is an issue with parsing the JSON or creating the DataFrame.
/// - Writing the Parquet file to disk fails.
///
async fn read_write_output(base: &str) -> Result<()> {
    let fname = Path::new(base).with_extension("json");
    eprintln!("Reading data from {:?}", fname);

    let fh = File::open(fname)?;
    let mut df = JsonReader::new(fh)
        .with_json_format(JsonFormat::JsonLines)
        .infer_schema_len(NonZeroUsize::new(3))
        .finish()?;
    eprintln!("{} records read", df.iter().len());

    // Prepare output
    //
    let fname = Path::new(base).with_extension("parquet");

    let mut file = File::create(fname)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;

    eprintln!("Done.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let fname = std::env::args().nth(1).ok_or("small").unwrap();

    let _ = read_write_output(&fname).await?;

    Ok(())
}
