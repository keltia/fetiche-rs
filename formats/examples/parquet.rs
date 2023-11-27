//! Read some data as json and write it into a parquet file
//!

use std::fs::File;
use std::string::ToString;

use eyre::Result;
use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::file::properties::EnabledStatistics;
use parquet::file::{properties::WriterProperties, writer::SerializedFileWriter};
use parquet::record::RecordWriter;
use tap::Tap;
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use fetiche_formats::Asd;

#[tracing::instrument]
fn read_write_output(base: &str) -> Result<()> {
    trace!("Read data.");

    let fname = format!("{}.json", base);
    trace!("fname={:?}", fname);

    let str = std::fs::read_to_string(&fname)?;
    trace!("Decode data.");

    let data: Vec<Asd> = serde_json::from_str(&str)?;
    let data: Vec<_> = data.iter().map(|r| r.fix_tm().unwrap()).collect();

    info!("{} records read", data.len());

    // Infer schema from data
    //
    let schema = data.as_slice().schema()?;
    trace!("schema={:?}", schema);

    // Prepare output
    //
    let fname = format!("{}.parquet", base);
    let file = File::create(&fname)?;

    let props = WriterProperties::builder()
        .set_created_by("fetiche".to_string())
        .set_encoding(Encoding::PLAIN)
        .set_statistics_enabled(EnabledStatistics::Page)
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    info!("Writing in {}", fname);
    let mut writer = SerializedFileWriter::new(file, schema.clone(), props.into())?;
    let mut row_group = writer.next_row_group()?;

    trace!("Writing data.");
    data.as_slice()
        .tap(|e| trace!("e={:?}", e))
        .write_to_row_group(&mut row_group)?;
    let m = row_group.close()?;
    info!("{} records written.", m.num_rows());

    writer.close()?;

    info!("Done.");
    Ok(())
}

const NAME: &str = "example.parquet";

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

    let fname = std::env::args().nth(1).ok_or("small").unwrap();

    let _ = read_write_output(&fname)?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
