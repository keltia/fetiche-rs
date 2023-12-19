//! Read some data as json and write it into a parquet file
//!
//! Alternative version using `arrow2` instead of arrow/parquet:etc.
//!

use std::fs::File;
use std::io::BufReader;

use arrow2::array::Array;
use arrow2::{
    chunk::Chunk,
    datatypes::Schema,
    io::parquet::write::{
        transverse, CompressionOptions, FileWriter, RowGroupIterator, Version, WriteOptions,
    },
};
use eyre::Result;
use parquet2::{compression::ZstdLevel, encoding::Encoding};
use serde::{Deserialize, Serialize};
use serde_arrow::schema::{SerdeArrowSchema, TracingOptions};
use serde_json::Deserializer;
use tracing::{debug, info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Cat21 {
    pub I010: I010,
    pub I040: I040,
    pub I161: I161,
    pub I015: I015,
    pub I071: Option<I071>,
    pub I130: I130,
    pub I131: I131,
    pub I072: Option<I072>,
    pub I150: Option<I150>,
    pub I151: Option<I151>,
    pub I080: I080,
    pub I073: I073,
    pub I074: Option<I074>,
    pub I075: I075,
    pub I076: Option<I076>,
    pub I140: I140,
    pub I090: I090,
    pub I210: I210,
    pub I070: I070,
    pub I230: Option<I230>,
    pub I145: I145,
    pub I152: Option<I152>,
    pub I200: I200,
    pub I155: I155,
    pub I157: Option<I157>,
    pub I160: I160,
    pub I165: Option<I165>,
    pub I077: I077,
    pub I170: I170,
    pub I020: I020,
    pub I220: Option<I220>,
    pub I146: I146,
    pub I148: Option<I148>,
    pub I110: Option<I110>,
    pub I016: I016,
    pub I008: I008,
    pub I271: Option<I271>,
    pub I132: Option<I132>,
    pub I250: Option<I250>,
    pub I260: Option<I260>,
    pub I400: I400,
    pub I295: I295,
    pub Ire: IRE,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I010 {
    pub sac: u8,
    pub sic: u8,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I040 {
    pub atp: u8,
    pub arc: u8,
    pub rc: u8,
    pub rab: u8,
    pub fx: u8,
    pub dcr: u8,
    pub gbs: u8,
    pub sim: u8,
    pub tst: u8,
    pub saa: u8,
    pub cl: u8,
    #[serde(rename = "spare")]
    pub spare: u8,
    pub llc: u8,
    pub ipc: u8,
    pub nogo: u8,
    pub cpr: u8,
    pub ldpj: u8,
    pub rcf: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I161 {
    pub spare: Option<u8>,
    #[serde(rename = "TrackN")]
    pub trackn: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I015 {
    pub id: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I071 {
    pub time_applicability_position: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct I130 {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct I131 {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I080 {
    #[serde(rename = "TAddr")]
    pub taddr: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I073 {
    pub time_reception_position: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I075 {
    pub time_reception_velocity: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I140 {
    pub geometric_heigth: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I090 {
    #[serde(rename = "NUCr_or_NACv")]
    pub nucr_or_nacv: u8,
    #[serde(rename = "NUCp_or_NIC")]
    pub nucp_or_nic: u8,
    #[serde(rename = "FX")]
    pub fx: u8,
    #[serde(rename = "NICbaro")]
    pub nicbaro: u8,
    #[serde(rename = "SIL")]
    pub sil: u8,
    #[serde(rename = "NACp")]
    pub nacp: u8,
    pub spare: u8,
    #[serde(rename = "SILS")]
    pub sils: u8,
    #[serde(rename = "SDA")]
    pub sda: u8,
    #[serde(rename = "GVA")]
    pub gva: u8,
    #[serde(rename = "PIC")]
    pub pic: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I210 {
    pub spare: u8,
    #[serde(rename = "VNS")]
    pub vns: u8,
    #[serde(rename = "VN")]
    pub vn: u8,
    #[serde(rename = "LTT")]
    pub ltt: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I070 {
    pub spare: u8,
    #[serde(rename = "Mode3A")]
    pub mode3a: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I145 {
    pub fl: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I200 {
    pub icf: u8,
    pub lnav: u8,
    pub me: u8,
    pub ps: u8,
    pub ss: u8,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I155 {
    pub re: u8,
    pub bvr: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I160 {
    pub re: u8,
    pub gs: f32,
    pub ta: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I077 {
    pub time_report_transmission: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I170 {
    pub tid: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I020 {
    pub ecat: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct I146 {
    #[serde(rename = "SAS")]
    pub sas: u8,
    #[serde(rename = "Source")]
    pub source: u8,
    #[serde(rename = "Alt")]
    pub alt: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I016 {
    pub rp: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I008 {
    pub ra: u8,
    pub tc: u8,
    pub ts: u8,
    pub arv: u8,
    pub cdti_a: u8,
    #[serde(rename = "not_TCAS")]
    pub not_tcas: u8,
    pub sa: u8,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I400 {
    pub rid: u8,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I295 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct IRE {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I072 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I150 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I151 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I074 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I076 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I230 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I152 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I157 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I165 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I220 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I148 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I110 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I271 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I132 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I250 {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct I260 {}

#[tracing::instrument]
fn read_json(base: &str) -> Result<(Schema, Vec<Box<dyn Array>>)> {
    trace!("Read data.");

    let fname = format!("{}.json", base);
    trace!("fname={:?}", fname);

    let topts = TracingOptions::default()
        .guess_dates(true)
        .allow_null_fields(true);

    let buf = BufReader::new(File::open(&fname)?);
    let json = Deserializer::from_reader(buf).into_iter::<Cat21>();

    let data: Vec<_> = json.map(|e| e.unwrap()).collect::<Vec<_>>();

    let data = data.as_slice();
    let fields = SerdeArrowSchema::from_samples(&data, topts)?.to_arrow2_fields()?;
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

    let fname = std::env::args().nth(1).unwrap_or("test".to_string());

    let (schema, data) = read_json(&fname)?;
    debug!("data={:?}", data);

    let _ = write_chunk(schema, data, &fname)?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
