//! Read some data as json and write it into a parquet file
//!
//! Alternative version using `arrow2` instead of arrow/parquet:etc.
//!

use std::fs::File;

use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use arrow2::io::json::read;
use arrow2::io::json::read::infer;
use arrow2::io::parquet::write::{
    transverse, CompressionOptions, Encoding, FileWriter, RowGroupIterator, Version, WriteOptions,
};
use chrono::NaiveDateTime;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Asd {
    /// Hidden UNIX timestamp
    #[serde(skip_deserializing)]
    pub tm: i64,
    /// Each record is part of a drone journey with a specific ID
    pub journey: u32,
    /// Identifier for the drone
    pub ident: String,
    /// Model of the drone
    pub model: Option<String>,
    /// Source ([see src/site/asd.rs]) of the data
    pub source: String,
    /// Point/record ID
    pub location: u32,
    /// Date of event (in the non standard YYYY-MM-DD HH:MM:SS formats)
    pub timestamp: String,
    /// $7 (actually f32)
    #[serde_as(as = "DisplayFromStr")]
    pub latitude: f32,
    /// $8 (actually f32)
    #[serde_as(as = "DisplayFromStr")]
    pub longitude: f32,
    /// Altitude, can be either null or negative (?)
    pub altitude: Option<i16>,
    /// Distance to ground (estimated every 15s)
    pub elevation: Option<i32>,
    /// Undocumented
    pub gps: Option<u32>,
    /// Signal level (in dB)
    pub rssi: Option<i32>,
    /// $13 (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub home_lat: Option<f32>,
    /// $14 (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub home_lon: Option<f32>,
    /// Altitude from takeoff point
    pub home_height: Option<f32>,
    /// Current speed
    pub speed: f32,
    /// True heading
    pub heading: f32,
    /// Name of detecting point
    pub station_name: Option<String>,
    /// Latitude (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub station_latitude: Option<f32>,
    /// Longitude (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub station_longitude: Option<f32>,
}

/// Generate a proper timestamp from the non-standard string they emit.
///
#[inline]
fn fix_tm(inp: Asd) -> Result<Asd> {
    let tod = NaiveDateTime::parse_from_str(&inp.timestamp, "%Y-%m-%d %H:%M:%S")?.timestamp();
    let mut out = inp.clone();
    out.tm = tod;
    Ok(out)
}

#[tracing::instrument]
fn read_json(base: &str) -> Result<(Box<dyn Array>, Schema)> {
    trace!("Read data.");

    let fname = format!("{}.json", base);
    trace!("fname={:?}", fname);
    let str = std::fs::read(&fname)?;

    trace!("Decode data.");
    let json = read::json_deserializer::parse(&str).unwrap();
    debug!("json={:?}", json);

    let data_type = infer(&json)?;

    let schema = read::infer_records_schema(&json)?;
    let res = read::deserialize(&json, data_type)?;

    Ok((res, schema))
}

#[tracing::instrument(skip(data, schema))]
fn write_output(data: Chunk<Box<dyn Array>>, schema: Schema, base: &str) -> Result<()> {
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(None),
        version: Version::V2,
        data_pagesize_limit: None,
    };

    let encodings: Vec<_> = schema
        .fields
        .iter()
        .map(|f| transverse(&f.data_type, |_| Encoding::Plain))
        .collect();
    trace!("encodings len={}", encodings.len());

    let iter = vec![Ok(data)];
    let row_groups = RowGroupIterator::try_new(iter.into_iter(), &schema, options, encodings)?;

    // Prepare output
    //
    let fname = format!("{}2.parquet", base);
    let file = File::create(&fname)?;

    let mut writer = FileWriter::try_new(file, schema, options)?;

    for group in row_groups {
        writer.write(group?)?;
    }
    let size = writer.end(None)?;

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

    let fname = std::env::args().nth(1).unwrap_or("small".to_string());

    let (data, schema) = read_json(&fname)?;
    debug!("data={:?}", data);
    debug!("schema={:?}", schema);

    let data = Chunk::new(vec![data]);

    let _ = write_output(data, schema, &fname)?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
