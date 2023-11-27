//! Read some data as json and write it into a parquet file
//!
//! Alternative version using `arrow2` instead of arrow/parquet:etc.
//!

use std::fs::File;
use std::vec;

use arrow2::array::Array;
use arrow2::{
    chunk::Chunk,
    datatypes::Schema,
    io::parquet::write::{
        transverse, CompressionOptions, FileWriter, RowGroupIterator, Version, WriteOptions,
    },
};
use arrow2_convert::serialize::TryIntoArrow;
use arrow2_convert::{ArrowDeserialize, ArrowField, ArrowSerialize};
use chrono::NaiveDateTime;
use eyre::Result;
use parquet2::encoding::Encoding;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[serde_as]
#[derive(ArrowField, ArrowSerialize, ArrowDeserialize, Clone, Debug, Deserialize, Serialize)]
pub struct Asd {
    /// Hidden UNIX timestamp
    #[serde(skip_deserializing)]
    pub time: i64,
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
fn fix_tm(inp: &Asd) -> Result<Asd> {
    let tod = NaiveDateTime::parse_from_str(&inp.timestamp, "%Y-%m-%d %H:%M:%S")?.timestamp();
    let mut out = inp.clone();
    out.time = tod;
    Ok(out)
}

#[tracing::instrument]
fn read_json(base: &str) -> Result<Vec<Asd>> {
    trace!("Read data.");

    let fname = format!("{}.json", base);
    trace!("fname={:?}", fname);
    let str = std::fs::read_to_string(&fname)?;

    trace!("Decode data.");
    let json: Vec<Asd> = serde_json::from_str(&str)?;
    let json = json.iter().map(|r| fix_tm(&r).unwrap()).collect();
    debug!("json={:?}", json);

    Ok(json)
}

#[tracing::instrument(skip(data))]
fn write_chunk(data: Vec<Asd>, base: &str) -> Result<()> {
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(None),
        version: Version::V2,
        data_pagesize_limit: None,
    };

    // Prepare output
    //
    let fname = format!("{}3.parquet", base);
    let file = File::create(&fname)?;

    // Prepare data
    //
    let arrow_array: Box<dyn Array> = data.try_into_arrow().unwrap();
    debug!("arrow_array={:?}", arrow_array);

    let struct_array = arrow_array
        .as_any()
        .downcast_ref::<arrow2::array::StructArray>()
        .unwrap();
    debug!("struct_array={:?}", struct_array);

    let fields = struct_array.fields().to_vec();
    debug!("fields={:?}", fields);

    let schema = Schema::from(fields);
    debug!("schema={:?}", schema);

    let encodings = schema
        .fields
        .iter()
        .map(|f| transverse(&f.data_type, |_| Encoding::Plain))
        .collect();

    let iter = vec![Ok(Chunk::new(vec![struct_array.clone().boxed()]))];
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

const NAME: &str = "parquet3";

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

    let data = read_json(&fname)?;
    debug!("data={:?}", data);

    let _ = write_chunk(data, &fname)?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
