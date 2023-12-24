//! Read some data as csv and write it into a parquet file
//!
//! Use `arrow2` in sync way.
//!
//! TODO: use `rayon`.
//!

use std::fs::File;

use arrow2::{
    array::Array,
    chunk::Chunk,
    datatypes::Schema,
    io::csv::read::{
        deserialize_batch, deserialize_column, infer, infer_schema, read_rows, ByteRecord,
        ReaderBuilder,
    },
    io::parquet::write::{
        transverse, CompressionOptions, FileWriter, RowGroupIterator, Version, WriteOptions,
    },
};

use eyre::Result;
use parquet2::{compression::ZstdLevel, encoding::Encoding};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

const BATCH_SIZE: usize = 500000;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Cat21 {
    #[serde(rename = "020.EmitterCategory")]
    pub ecat: u8,
    #[serde(rename = "040.GBS")]
    pub gbs: u8,
    #[serde(rename = "070.ModeA")]
    pub mode3a: String,
    #[serde(rename = "073.TimeRecPosition")]
    pub time_rec_position: f32,
    #[serde(rename = "080.AircraftAddress")]
    pub aircraft_addr: String,
    #[serde(rename = "131.Latitude")]
    pub latitude: f32,
    #[serde(rename = "131.Longitude")]
    pub longitude: f32,
    #[serde(rename = "140.GeometricAltitude")]
    pub geometric_altitude: f32,
    #[serde(rename = "145.FlightLevel")]
    pub flight_level: f32,
    #[serde(rename = "155.BarometricVerticalRate")]
    pub barometric_vertical_rate: f32,
    #[serde(rename = "157.RE")]
    pub re: Option<String>,
    #[serde(rename = "157.GeometricVerticalRate")]
    pub geometric_vertical_rate: f32,
    #[serde(rename = "160.GroundSpeed")]
    pub ground_speed: f32,
    #[serde(rename = "160.TrackAngle")]
    pub track_angle: f32,
    #[serde(rename = "170.Callsign")]
    pub callsign: String,
    #[serde(rename = "R")]
    pub r: Option<SGV>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SGV {
    #[serde(rename = "STP")]
    pub stp: Option<String>,
    #[serde(rename = "HTS")]
    pub hts: Option<String>,
    #[serde(rename = "HTT")]
    pub htt: Option<String>,
    #[serde(rename = "HRD")]
    pub hrd: Option<String>,
    #[serde(rename = "GSS")]
    pub gss: Option<String>,
    #[serde(rename = "HGT")]
    pub hgt: Option<String>,
}

#[tracing::instrument]
fn read_csv(base: &str) -> Result<(Schema, Vec<Chunk<Box<dyn Array>>>)> {
    trace!("Read data.");

    let fname = format!("{}.csv", base);
    trace!("fname={:?}", fname);

    let mut reader = ReaderBuilder::new().from_path(&fname)?;
    let (fields, _) = infer_schema(&mut reader, None, true, &infer)?;
    let schema = Schema::from(fields.clone());

    // Batch size = 10000
    //
    let mut size = 1;
    let mut data = vec![];
    while size > 0 {
        let mut rows = vec![ByteRecord::default(); BATCH_SIZE];
        let rows_read = read_rows(&mut reader, 0, &mut rows)?;
        info!("{} rows read.", rows_read);

        let rows = &rows[..rows_read];
        size = rows.len();

        if size > 0 {
            let chunk = deserialize_batch(rows, &fields, None, 0, deserialize_column)?;
            debug!("arrays={:?}", chunk);

            data.push(chunk)
        }
    }
    info!("{} batches.", data.len());
    Ok((schema, data))
}

#[tracing::instrument(skip(data))]
fn write_chunk(schema: Schema, data: Vec<Chunk<Box<dyn Array>>>, base: &str) -> Result<()> {
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

    info!("Done.");
    Ok(())
}

const NAME: &str = "adsb-ff";

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

    let fname = std::env::args().nth(1).unwrap_or("test".to_string());

    let (schema, data) = read_csv(&fname)?;
    debug!("data={:?}", data);

    let _ = write_chunk(schema, data, &fname)?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
