//! Read some data as csv and write it into a parquet file
//!

use crate::cli::Opts;
use adsb_to_parquet::{
    arrow2::{read_csv, write_chunk},
    datafusion::parquet_through_df,
    Options,
};
use clap::Parser;
use eyre::Result;
use fetiche_common::init_logging;
use std::path::Path;
use tracing::{debug, trace};

mod cli;
mod types;

// Name of the application for the parquet header
//
const NAME: &str = "adsb-to-parquet";

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    init_logging(NAME, false, true, Some("".to_string()))?;
    trace!("Logging initialised.");

    // Generate our basename
    //
    let input = opts.name.clone();

    // Extract basename and add ".parquet"
    //
    let base = String::from(Path::new(&opts.name).file_name().unwrap().to_string_lossy());
    trace!("Using {} as basename", base);

    let output = opts.output.unwrap_or(format!("{}.parquet", base));

    // nh = no header line (default = false which means has header line).
    //
    let header = !opts.nh;
    let delim = opts.delim.clone().as_bytes()[0];
    let opt = Options { delim, header };

    eprintln!(
        "Reading {} with {} as delimiter",
        base,
        String::from_utf8(vec![opt.delim])?
    );

    // arrow2 or datafusion?
    //
    if opts.arrow2 {
        let (schema, data) = read_csv(&input, opt)?;
        debug!("data={:?}", data);

        eprintln!("Writing to {}", output);
        let tm = write_chunk(schema, data, &output)?;
        eprintln!("Done in {}ms.", tm);
    } else {
        // This is async
        //
        parquet_through_df(&input, &output, opt).await?;
    }
    Ok(())
}
