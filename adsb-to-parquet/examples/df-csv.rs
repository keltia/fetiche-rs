//! Read some data as csv and write it into a parquet file
//!
//! Use `arrow2` in sync way.
//!
//! TODO: use `rayon`.
//!

use std::path::Path;
use std::time::Instant;

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::parquet::basic::{Compression, Encoding, ZstdLevel};
use datafusion::parquet::file::properties::{EnabledStatistics, WriterProperties};
use datafusion::prelude::*;
use eyre::Result;
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Has headers or not?
    #[clap(short = 'N', long = "no-header")]
    pub nh: bool,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Delimiter for csv files.
    #[clap(short, default_value = ",")]
    pub delim: String,
    /// Filename, can be just the basename and .csv/.parquet are implied
    pub name: String,
}

const NAME: &str = "df-csv";

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
    let fname = if opts.name.ends_with(".csv") {
        String::from(Path::new(&opts.name).file_name().unwrap().to_string_lossy())
    } else {
        opts.name
    };
    trace!("Using {} as basename", fname);

    // nh = no header line (default = false which means has header line).
    //
    let header = !opts.nh;
    let delim = opts.delim.clone().as_bytes()[0];

    let ctx = SessionContext::new();
    let copts = CsvReadOptions::new().delimiter(delim).has_header(header);

    eprintln!(
        "Reading {}.csv with {} as delimiter",
        fname,
        String::from_utf8(vec![delim])?
    );

    let start = Instant::now();
    let df = ctx.read_csv(&fname, copts).await?;
    let tm = (Instant::now() - start).as_millis() as u64;

    eprintln!("Read {} records in {}ms", df.clone().count().await?, tm);

    let fname = opts.output.unwrap_or(fname);

    let dopts = DataFrameWriteOptions::default().with_single_file_output(true);
    let props = WriterProperties::builder()
        .set_created_by(NAME.to_string())
        .set_encoding(Encoding::PLAIN)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_compression(Compression::ZSTD(ZstdLevel::try_new(8)?))
        .build();

    eprintln!("Writing to {}.", fname);
    let start = Instant::now();
    let _ = df.write_parquet(&fname, dopts, Some(props)).await?;
    let tm = (Instant::now() - start).as_millis() as u64;
    eprintln!("Done in {}ms.", tm);

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
