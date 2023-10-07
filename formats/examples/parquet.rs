//! Read some data as json and write it into a parquet file
//!

use std::fs;
use std::fs::File;
use std::string::ToString;

use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::{
    file::{properties::WriterProperties, writer::SerializedFileWriter},
    record::RecordWriter,
};
use parquet_derive::ParquetRecordWriter;
use serde::{Deserialize, Serialize};
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[derive(Clone, Debug, Deserialize, ParquetRecordWriter, Serialize)]
pub struct Asd {
    // Each record is part of a drone journey with a specific ID
    pub journey: u32,
    // Identifier for the drone
    pub ident: String,
    // Model of the drone
    pub model: Option<String>,
    // Source ([see src/site/asd.rs]) of the data
    pub source: String,
    // Point/record ID
    pub location: u32,
    // Date of event (in the non standard YYYY-MM-DD HH:MM:SS formats)
    pub timestamp: String,
    // $7 (actually f32)
    pub latitude: String,
    // $8 (actually f32)
    pub longitude: String,
    // Altitude, can be either null or negative (?)
    pub altitude: Option<i16>,
    // Distance to ground (estimated every 15s)
    pub elevation: Option<u32>,
    // Undocumented
    pub gps: Option<u32>,
    // Signal level (in dB)
    pub rssi: Option<i32>,
    // $13 (actually f32)
    pub home_lat: Option<String>,
    // $14 (actually f32)
    pub home_lon: Option<String>,
    // Altitude from takeoff point
    pub home_height: Option<f32>,
    // Current speed
    pub speed: f32,
    // True heading
    pub heading: f32,
    // Name of detecting point
    pub station_name: Option<String>,
    // Latitude (actually f32)
    pub station_latitude: Option<String>,
    // Longitude (actually f32)
    pub station_longitude: Option<String>,
}

#[tracing::instrument]
fn main() -> eyre::Result<()> {
    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_targets(true)
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

    trace!("Read data.");
    let str = fs::read_to_string("asd.json")?;
    let data: Vec<Asd> = serde_json::from_str(&str).unwrap();

    info!("{} records read", data.len());

    // Infer schema from data
    //
    let schema = data.as_slice().schema()?;

    trace!("Prepare output");
    // Prepare output
    //
    let file = File::create("asd.parquet")?;
    let props = WriterProperties::builder()
        .set_created_by("fetiche".to_string())
        .set_encoding(Encoding::PLAIN)
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    let mut writer = SerializedFileWriter::new(file, schema, props.into())?;
    let mut row_group = writer.next_row_group()?;

    data.as_slice().write_to_row_group(&mut row_group)?;

    trace!("Done.");
    Ok(())
}
