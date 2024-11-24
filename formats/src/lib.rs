//! Definition of a data formats
//!
//! This module makes the link between the shared output formats `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new formats, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input formats and the transformations needed.
//!

use chrono::{DateTime, Utc};
use csv::WriterBuilder;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Debug;
use strum::EnumString;
use tabled::{builder::Builder, settings::Style};
use tracing::trace;

// Re-export for convenience
//
pub use aeroscope::*;
pub use asd::*;
pub use asterix::*;
pub use avionix::*;
#[cfg(feature = "flightaware")]
pub use flightaware::*;
pub use opensky::*;
#[cfg(feature = "safesky")]
pub use safesky::*;

mod aeroscope;
mod asd;
mod asterix;
mod avionix;
#[cfg(feature = "flightaware")]
mod flightaware;
mod opensky;
#[cfg(feature = "safesky")]
mod safesky;
pub mod senhive;

/// Current formats.hcl version
///
const FVERSION: usize = 2;

// -----

pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// For each format, we define a set of key attributes that will get displayed.
///
#[derive(Debug, Deserialize)]
pub struct FormatDescr {
    /// Type of data each format refers to
    #[serde(rename = "type")]
    pub dtype: String,
    /// Free text description
    pub description: String,
    /// Source
    pub source: String,
    /// URL to the site where this is defined
    pub url: String,
}

/// Struct to be read from an HCL file at compile-time
///
#[derive(Debug, Deserialize)]
pub struct FormatFile {
    /// Version
    pub version: usize,
    /// Ordered list of format metadata
    pub format: BTreeMap<String, FormatDescr>,
}

/// WWe distinguish between the site-specific data formats and general ADS-B
///
#[derive(Clone, Debug, Deserialize, strum::Display, EnumString, strum::VariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum DataType {
    /// ADS-B data
    Adsb,
    /// Drone data, site-specific
    Drone,
    /// Write formats
    Write,
}

/// This struct holds the different data formats that we support.
///
#[derive(
    Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq, strum::Display, EnumString, Serialize,
)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum Format {
    #[default]
    None,
    /// Special cut-down version of ADS-B, limited to specific fields
    Adsb21,
    /// DJI Aeroscope-specific data, coming from the antenna
    Aeroscope,
    /// Consolidated drone data, from airspacedrone.com (ASD)
    Asd,
    /// Aero Network JSON format by Avionix for drones
    CubeData,
    /// ADS-B data from the Avionix appliance
    AvionixCat21,
    /// ECTL Asterix Cat21 flattened CSV
    Cat21,
    /// ECTL Drone specific Asterix Cat129
    Cat129,
    /// Flightaware API v4 Position data
    Flightaware,
    /// ADS-B data from the Opensky API
    Opensky,
    /// Opensky data from the Impala historical DB
    PandaStateVector,
    /// ADS-B data  from the Safesky API
    Safesky,
    /// Drone data from Thales Senhive API
    Senhive,
}

/// This is the special hex string for ICAO codes
///
pub type ICAOString = [u8; 6];

/// Generate a converter called `$name` which takes `&str` and
/// output a `Vec<$to>`.  `input` is deserialized from JSON as
/// `$from`.
///
/// Uses `$to::from()` for each format.
///
/// You will need to `use` these in every file you use the macro
/// ```no_run
/// use eyre::Result;
/// use log::debug;
/// ```
/// or
/// ```no_run
/// use eyre::Result;
/// use tracing::debug;
/// ```
///
/// Takes 3 arguments:
///
/// - name of the `fn` to create
/// - name of the input `struct`
/// - name of the output type like `Cat21`
///
#[macro_export]
macro_rules! convert_to {
    ($name:ident, $from:ident, $to:ident) => {
        impl $to {
            #[doc = concat!("This is ", stringify!($name), " which convert a json string into a ", stringify!($to), "object")]
            ///
            #[tracing::instrument]
            pub fn $name(input: &str) -> Result<Vec<$to>> {
                debug!("IN={:?}", input);
                let stream = ::std::io::BufReader::new(input.as_bytes());
                let res = ::serde_json::Deserializer::from_reader(stream).into_iter::<$from>();

                let res: Vec<_> = res
                    .filter(|l| l.is_ok())
                    .enumerate()
                    .inspect(|(n, f)| debug!("cnt={}/{:?}", n, f.as_ref().unwrap()))
                    .map(|(_cnt, rec)| {
                        $to::from(&rec.unwrap())
                    })
                    .collect();
                debug!("res={:?}", res);
                Ok(res)
            }
        }
    };
}

impl Format {
    /// List all supported formats into a string using `tabled`.
    ///
    pub fn list() -> Result<String> {
        let descr = include_str!("formats.hcl");
        let fstr: FormatFile = hcl::from_str(descr)?;

        // Safety checks
        //
        assert_eq!(fstr.version, FVERSION);

        let header = vec!["Name", "Type", "Description"];

        let mut builder = Builder::default();
        builder.push_record(header);

        fstr.format.iter().for_each(|(name, entry)| {
            let mut row = vec![];

            let name = name.clone();
            let dtype = entry.dtype.clone();
            let description = entry.description.clone();
            let source = entry.source.clone();
            let url = entry.url.clone();

            let row_text = format!("{}\nSource: {} -- URL: {}", description, source, url);
            let dtype = dtype.to_string();
            row.push(&name);
            row.push(&dtype);
            row.push(&row_text);
            builder.push_record(row);
        });
        let allf = builder.build().with(Style::modern()).to_string();
        let str = format!("List all formats:\n{allf}");
        Ok(str)
    }

    /// List all supported formats into a string
    ///
    pub fn list_plain() -> Result<String> {
        let descr = include_str!("formats.hcl");
        let fstr: FormatFile = hcl::from_str(descr)?;
        assert_eq!(fstr.version, FVERSION);
        let allf = fstr
            .format
            .iter()
            .map(|(name, entry)| {
                format!(
                    "{:10}{:6}{}\n{:16}Source: {} -- URL: {}",
                    name, entry.dtype, entry.description, "", entry.source, entry.url
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let str = format!("List all formats:\n\n{allf}");
        Ok(str)
    }
}

/// This structure hold a general location object with lat/long.
///
/// In CSV files, the two fields are merged into this struct on deserialization
/// and used as-is when coming from JSON.
///
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Position {
    // Latitude in degrees
    pub latitude: f32,
    /// Longitude in degrees
    pub longitude: f32,
}

impl Default for Position {
    fn default() -> Self {
        Position {
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}

// ----- New `DronePoint`, flattened struct

/// This is a flattened struct trying to gather all elements we can find in a given drone-related
/// feed (Avionix, Senhive) into a common type: `DronePoint`.
///
/// It mimics the `[Asd`](../asd.rs) struct.
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DronePoint {
    /// timestamp
    pub time: DateTime<Utc>,
    /// Each record is part of a drone journey with a specific ID
    pub journey: String,
    /// Identifier for the drone
    pub ident: Option<String>,
    /// Maker of the drone
    pub make: Option<String>,
    /// Model of the drone
    pub model: Option<String>,
    /// UAV Type
    pub uav_type: u8,
    /// Source
    pub source: u8,
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
    /// Geodesic aka true altitude
    pub altitude: Option<f64>,
    /// Distance to ground
    pub elevation: Option<f64>,
    /// Operator lat
    pub home_lat: Option<f64>,
    /// Operator lon
    pub home_lon: Option<f64>,
    /// Altitude from takeoff point
    pub home_height: Option<f64>,
    /// Current speed
    pub speed: f64,
    /// True heading
    pub heading: f64,
    /// Vehicle state
    pub state: Option<u8>,
    /// Name of detecting point -- system.fusion_state.source_serials
    pub station_name: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum VehicleStateType {
    MotorOff = 0,
    MotorOn = 1,
    Airborn = 2,
    #[default]
    Unknown = 15,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[repr(u8)]
pub enum FusionType {
    Cooperative = 0,
    Surveillance = 1,
    Both = 2,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[repr(u8)]
pub enum UAVType {
    #[default]
    Unknown = 0,
    FixedWing = 1,
    MultiRotor = 2,
    Gyroplane = 3,
    HybridLift = 4,
    Other = 15,
}

/// Special enum for type of source
///
#[derive(Debug, Deserialize, Serialize, strum::Display, EnumString, strum::VariantNames)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Source {
    /// ADS-B
    A,
    /// MLAT
    M,
    /// UAT,
    U,
    /// ADS-L
    L,
    /// FLARM
    F,
    /// OGN
    O,
    /// Remote-ID
    Rid,
    /// 4G/5G
    Lte,
    /// PilotAware
    P,
    /// FANET
    N,
    /// Asterix
    X,
}

impl From<Source> for u8 {
    fn from(value: Source) -> Self {
        match value {
            Source::A => 0,
            Source::M => 1,
            Source::U => 2,
            Source::L => 3,
            Source::F => 4,
            Source::O => 5,
            Source::Rid => 6,
            Source::Lte => 7,
            Source::P => 8,
            Source::N => 9,
            Source::X => 10,
        }
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TodCalculated {
    C,
    L,
    #[default]
    N,
    R,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Bool {
    Y,
    #[default]
    N,
}

/// Convert into feet
///
#[inline]
pub fn to_feet(a: f32) -> u32 {
    (a * 3.28084) as u32
}

/// Convert into knots
///
#[inline]
pub fn to_knots(a: f32) -> f32 {
    a * 0.54
}

/// Convert to meters
///
#[inline]
pub fn to_meters(a: f32) -> f32 {
    a / 3.28084
}

/// Output the final csv file with a different delimiter 'now ":")
///
#[tracing::instrument]
pub fn prepare_csv<T>(data: Vec<T>, header: bool) -> Result<String>
where
    T: Serialize + Debug,
{
    trace!("Generating output…");
    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new()
        .delimiter(b':')
        .has_headers(header)
        .from_writer(vec![]);

    // Insert data
    //
    data.iter().for_each(|rec| {
        wtr.serialize(rec).unwrap();
    });

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_default() {
        let s = Format::default();

        assert_eq!(Format::None, s);
    }

    #[test]
    fn test_to_feet() {
        assert_eq!(1, to_feet(0.305))
    }

    #[test]
    fn test_to_knots() {
        assert_eq!(1.00008, to_knots(1.852))
    }

    #[test]
    fn test_position_default() {
        let p = Position::default();
        assert_eq!(
            Position {
                latitude: 0.0,
                longitude: 0.0,
            },
            p
        );
    }
}
