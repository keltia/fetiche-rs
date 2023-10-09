//! Read some data as json and write it into a parquet file
//!

use std::fs;
use std::fs::File;
use std::string::ToString;

use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::schema::types::TypePtr;
use parquet::{
    file::{properties::WriterProperties, writer::SerializedFileWriter},
    record::RecordWriter,
};
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use fetiche_formats::Asd;

#[tracing::instrument]
fn read_data(fname: &str) -> eyre::Result<Vec<Asd>> {
    trace!("Read data.");
    let str = fs::read_to_string(fname)?;
    trace!("Decode data.");
    let data: Vec<Asd> = serde_json::from_str(&str)?;
    Ok(data)
}

#[tracing::instrument(skip(data, schema))]
fn write_output(schema: TypePtr, data: &Vec<Asd>) -> eyre::Result<()> {
    // Prepare output
    //
    let file = File::create(OUTPUT)?;
    let props = WriterProperties::builder()
        .set_created_by("fetiche".to_string())
        .set_encoding(Encoding::PLAIN)
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    info!("Writing in {}", OUTPUT);
    let mut writer = SerializedFileWriter::new(file, schema, props.into())?;
    let mut row_group = writer.next_row_group()?;

    trace!("Writing data.");
    let _ = data.as_slice().write_to_row_group(&mut row_group)?;
    trace!("Done.");
    Ok(())
}

const INPUT: &str = "asd.json";
const OUTPUT: &str = "asd.parquet";

#[tracing::instrument]
fn main() -> eyre::Result<()> {
    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_thread_names(true)
        .with_span_retrace(true)
        .with_targets(true)
        .with_verbose_entry(true)
        .with_verbose_exit(true)
        .with_higher_precision(true)
        .with_bracketed_fields(true);

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .init();
    trace!("Logging initialised.");

    let data = read_data(INPUT)?;

    info!("{} records read", data.len());

    // Infer schema from data
    //
    let schema = data.as_slice().schema()?;

    trace!("Prepare output");
    let _ = write_output(schema, &data);

    Ok(())
}
