//! Read some data as csv and write it into a parquet file
//!
//! Use `arrow2` in sync way.
//!
//! TODO: use `rayon`.
//!

use std::fs::File;
use std::path::Path;
use std::time::Instant;

use arrow2::{
    array::Array,
    chunk::Chunk,
    datatypes::Schema,
    io::csv::read::{
        deserialize_column, infer, infer_schema, read_rows, ByteRecord, ReaderBuilder,
    },
    io::parquet::write::{
        transverse, CompressionOptions, FileWriter, RowGroupIterator, Version, WriteOptions,
    },
};
use clap::Parser;
use eyre::Result;
use parquet2::{compression::ZstdLevel, encoding::Encoding};
use rayon::prelude::*;
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use crate::cli::Opts;

mod cli;
mod types;

const BATCH_SIZE: usize = 500000;

#[derive(Debug)]
struct Options {
    pub delim: u8,
    pub header: bool,
}

#[tracing::instrument]
fn read_csv(base: &str, opt: Options) -> Result<(Schema, Vec<Chunk<Box<dyn Array>>>)> {
    trace!("Read data.");

    let fname = format!("{}.csv", base);
    trace!("fname={:?}", fname);

    let mut reader = ReaderBuilder::new()
        .delimiter(opt.delim)
        .from_path(&fname)?;
    let (fields, _) = infer_schema(&mut reader, None, opt.header, &infer)?;
    let schema = Schema::from(fields.clone());

    // Read in batches of `BATCH_SIZE` elements.
    //
    let mut total = 0;
    let mut data = vec![];

    // Fill in with input data
    //
    loop {
        let mut rows = vec![ByteRecord::default(); BATCH_SIZE];
        let rows_read = read_rows(&mut reader, 0, &mut rows)?;

        // Are we finished?
        if rows_read == 0 {
            break;
        }
        let rows = &rows[..rows_read];

        // Count by lines, not bytes.
        //
        let size = rows.len();

        total += size;

        // Now process all columns in parallel
        //
        let arrays: Vec<Box<dyn Array>> = fields
            .par_iter()
            .enumerate()
            .map(|(n, field)| deserialize_column(rows, n, field.data_type.clone(), 0).unwrap())
            .collect();

        let chunk = Chunk::new(arrays);
        debug!("arrays={:?}", chunk);

        data.push(chunk);
    }
    info!("{} lines in {} batches.", total, data.len());

    Ok((schema, data))
}

#[tracing::instrument(skip(schema, data))]
fn write_chunk(schema: Schema, data: Vec<Chunk<Box<dyn Array>>>, base: &str) -> Result<u64> {
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(Some(ZstdLevel::try_new(8)?)),
        version: Version::V2,
        data_pagesize_limit: None,
    };

    debug!("data in={:?}", data);

    // Prepare output
    //
    let fname = format!("{}.parquet", base);
    let file = File::create(&fname)?;

    let start = Instant::now();
    let iter: Vec<_> = data.iter().map(|e| Ok(e.clone())).collect();
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
    info!("{} bytes written.", size);

    let tm = Instant::now() - start;
    Ok(tm.as_millis() as u64)
}

const NAME: &str = "adsb-to-parquet";

fn main() -> Result<()> {
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
    let opt = Options { delim, header };

    eprintln!(
        "Reading {}.csv with {} as delimiter",
        fname,
        String::from_utf8(vec![opt.delim])?
    );
    let (schema, data) = read_csv(&fname, opt)?;
    debug!("data={:?}", data);

    let fname = opts.output.unwrap_or(fname);

    eprintln!("Writing to {}.parquet", fname);
    let tm = write_chunk(schema, data, &fname)?;
    eprintln!("Done in {}ms.", tm);

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
