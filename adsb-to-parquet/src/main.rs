//! Read some data as csv and write it into a parquet file
//!
use std::path::Path;

use clap::Parser;
use eyre::Result;
use tracing::{debug, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use adsb_to_parquet::{
    arrow2::{read_csv, write_chunk},
    datafusion::parquet_through_df,
    Options,
};

use crate::cli::Opts;

mod cli;
mod types;

// Name of the application for the parquet header
//
const NAME: &str = "adsb-to-parquet";

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_span_retrace(true)
        .with_targets(true)
        .with_verbose_entry(true)
        .with_verbose_exit(true)
        .with_higher_precision(true)
        .with_bracketed_fields(true);

    // Setup Open Telemetry with Jaeger
    //
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_auto_split_batch(true)
        .with_max_packet_size(9_216)
        .with_service_name(NAME)
        .install_simple()?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .with(telemetry)
        .init();
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
    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
