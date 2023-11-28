//! Read some data as json and write it into a parquet file
//!
//! Alternative version using `arrow2` instead of arrow/parquet:etc.
//!

use std::fs::File;
use std::io::{BufRead, BufReader, Seek};

use arrow2::array::Array;
use arrow2::io::ndjson::read;
use arrow2::{
    chunk::Chunk,
    datatypes::Schema,
    io::parquet::write::{
        transverse, CompressionOptions, FileWriter, RowGroupIterator, Version, WriteOptions,
    },
};
use eyre::Result;
use parquet2::compression::ZstdLevel;
use parquet2::encoding::Encoding;
use serde_arrow::schema::TracingOptions;
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

const BATCH: usize = 20;

#[tracing::instrument]
fn read_json(base: &str) -> Result<(Schema, Vec<Box<dyn Array>>)> {
    trace!("Read data.");

    let fname = format!("{}.json", base);
    trace!("fname={:?}", fname);

    let mut reader = BufReader::new(File::open(fname)?);

    let dt = read::infer(&mut reader, None)?;
    reader.rewind()?;
    debug!("dt={:?}", dt);

    let mut reader = read::FileReader::new(reader, vec!["".to_string(); BATCH], None);
    let mut arrays = vec![];

    while let Some(rows) = reader.next()? {
        let array = read::deserialize(rows, dt.clone())?;
        arrays.push(array);
    }

    trace!("Decode data.");

    let schema = infer_records_schema(&dt)?;
    debug!("schema={:?}", schema);

    let json = read::deserialize(&json, dt)?;
    debug!("json={:?}", json);

    // Patch tm inside every record
    //
    //let json = json.iter().map(|r| r.fix_tm().unwrap()).collect();

    Ok((schema, json))
}

#[tracing::instrument(skip(data))]
fn write_chunk(schema: Schema, data: Box<dyn Array>, base: &str) -> Result<()> {
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

    // Prepare data
    //
    let topts = TracingOptions::default()
        .allow_null_fields(true)
        .guess_dates(true);

    let dt = data.data_type();
    debug!("dt={:?}", dt);

    let iter = vec![Ok(Chunk::new(vec![data]))];
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
