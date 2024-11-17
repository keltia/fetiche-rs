//! Read some data as json and write it into a parquet file
//!

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::num::NonZeroUsize;
use std::path::Path;

use eyre::Result;
use polars::prelude::{JsonFormat, JsonLineReader, JsonReader, ParquetWriter, SerReader};
use polars_core::prelude::*;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use fetiche_common::init_logging;

#[tracing::instrument]
async fn read_write_output(base: &str) -> Result<()> {
    trace!("Read data.");

    let fname = Path::new(base).with_extension("json");
    trace!("fname={:?}", fname);

    let fh = File::open(fname)?;
    let mut df = JsonReader::new(fh)
        .with_json_format(JsonFormat::JsonLines)
        .infer_schema_len(NonZeroUsize::new(3))
        .finish()?;
    info!("{} records read", df.clone().count().await?);

    // Prepare output
    //
    let fname = Path::new(base).with_extension("parquet");

    let mut file = File::create(fname)?;
    ParquetWriter::new(&mut file).finish(&mut df)?;

    info!("Done.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise logging early
    //
    init_logging("parquet", false, true, None)?;
    trace!("Logging initialised.");

    let fname = std::env::args().nth(1).ok_or("small").unwrap();

    let _ = read_write_output(&fname).await?;

    Ok(())
}
