//! Read some data as json and write it into a parquet file
//!
//! Alternative version using `arrow2` instead of arrow/parquet:etc.
//!
//! Benchmarks (hyperfine): 55990279 bytes, 136373 records  -- Mac Studio M2 Pro, dev builds
//!
//! Uncompressed size:
//! Zstd, default compression 3
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     10.393 s ±  0.031 s    [User: 9.965 s, System: 0.168 s]
//!   Range (min … max):   10.361 s … 10.456 s    10 runs
//! ```
//! Size: 2881674
//!
//! Zstd, compression 8
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     11.163 s ±  0.033 s    [User: 10.726 s, System: 0.175 s]
//!   Range (min … max):   11.131 s … 11.225 s    10 runs
//! ```
//! Size: 2558422
//!
//! Brotli, default level
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     11.197 s ±  0.196 s    [User: 10.755 s, System: 0.167 s]
//!   Range (min … max):   11.010 s … 11.517 s    10 runs
//! ```
//! Size: 2943996
//!
//! For fun:
//! Zstd, max compression level 22
//! ```text
//! Benchmark 1: cargo run --example parquet25 2023-jul-nov2
//!   Time (mean ± σ):     35.282 s ±  0.304 s    [User: 33.202 s, System: 1.802 s]
//!   Range (min … max):   35.044 s … 36.099 s    10 runs
//! ```
//! Size: 2048914

use std::fs::File;
use std::io::BufReader;

use arrow2::array::Array;
use arrow2::datatypes::Field;
use arrow2::{
    chunk::Chunk,
    datatypes::Schema,
    io::parquet::write::{
        transverse, CompressionOptions, FileWriter, RowGroupIterator, Version, WriteOptions,
    },
};
use eyre::Result;
use parquet2::{compression::ZstdLevel, encoding::Encoding};
use serde_arrow::schema::{SchemaLike, TracingOptions};
use serde_json::Deserializer;
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use fetiche_formats::Asd;

#[tracing::instrument]
fn read_json(base: &str) -> Result<(Schema, Vec<Box<dyn Array>>)> {
    trace!("Read data.");

    let fname = format!("{}.json", base);
    trace!("fname={:?}", fname);

    let topts = TracingOptions::default()
        .guess_dates(true)
        .allow_null_fields(true);

    let buf = BufReader::new(File::open(&fname)?);
    let json = Deserializer::from_reader(buf).into_iter::<Asd>();

    let data: Vec<_> = json
        .map(|e| e.unwrap().fix_tm().unwrap())
        .collect::<Vec<_>>();

    let data = data.as_slice();
    let fields = Vec::<Field>::from_samples(&data, topts)?;
    trace!("fields={:?}", fields);

    let schema = Schema::from(fields.clone());
    debug!("schema={:?}", schema);

    let arrays = serde_arrow::to_arrow2(&fields, &data)?;
    debug!("arrays={:?}", arrays);

    Ok((schema, arrays))
}

#[tracing::instrument(skip(data))]
fn write_chunk(schema: Schema, data: Vec<Box<dyn Array>>, base: &str) -> Result<()> {
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(Some(ZstdLevel::try_new(8)?)),
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

const NAME: &str = "parquet2";

fn main() -> Result<()> {
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

    let fname = std::env::args().nth(1).unwrap_or("small".to_string());

    let (schema, data) = read_json(&fname)?;
    debug!("data={:?}", data);

    let _ = write_chunk(schema, data, &fname)?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
