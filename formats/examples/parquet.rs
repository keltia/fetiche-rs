//! Read some data as json and write it into a parquet file
//!

use std::fs::File;
use std::path::Path;
use std::string::ToString;

use eyre::Result;
use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::schema::types::TypePtr;
use parquet::{
    file::{properties::WriterProperties, writer::SerializedFileWriter},
    record::RecordWriter,
};
use tap::Tap;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use fetiche_formats::Asd;

#[tracing::instrument]
async fn read_data(fname: &str) -> Result<Vec<Asd>> {
    trace!("Read data.");
    let fname = format!("{}.json", fname);
    trace!("fname={:?}", fname);
    let str = tokio::fs::read_to_string(fname).await?;
    trace!("Decode data.");
    let data: Vec<Asd> = serde_json::from_str(&str)?;
    Ok(data)
}

#[tracing::instrument(skip(data, schema))]
async fn write_output(fname: &str, schema: TypePtr, data: &Vec<Asd>) -> Result<()> {
    // Prepare output
    //
    let fname = format!("{}.parquet", fname);
    let file = File::create(&fname)?;
    let props = WriterProperties::builder()
        .set_created_by("fetiche".to_string())
        .set_encoding(Encoding::PLAIN)
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    info!("Writing in {}", fname);
    let mut writer = SerializedFileWriter::new(file, schema, props.into())?;
    let mut row_group = writer.next_row_group()?;

    trace!("Writing data.");
    let _ = data
        .as_slice()
        .tap(|e| trace!("e={:?}", e))
        .write_to_row_group(&mut row_group)?;
    //writer.close()?;
    trace!("Done.");
    Ok(())
}

const INPUT: &str = "asd";
const NAME: &str = "example.parquet";

#[tokio::main]
async fn main() -> Result<()> {
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

    let fname = std::env::args().nth(1).ok_or("small").unwrap();

    let data = read_data(&fname).await?;

    info!("{} records read", data.len());

    // Infer schema from data
    //
    let schema = data.as_slice().schema()?;

    trace!("Prepare output");
    let _ = write_output(&fname, schema, &data).await;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
