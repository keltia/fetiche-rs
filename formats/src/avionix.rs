//! This is the pseudo-cat21 file format specified in the Avionix documentation.
//!
//! URL: http://www.avionix.pl
//!

use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::{EnumString, EnumVariantNames};

/// This format is sent through a CSV file and has the following fields:
///
/// - UTI: UNIX timestamp in milli-secs (u64)
/// - DAT: ESRI timestamp e.g. 2015-07-26 07:36:51.657189000
/// - SIC
/// - SAC
/// - HEX: ICAO 6 byte code for the aircraft.
/// - FLI: Call-sign
/// - LAT: Latitude (WGS-84)
/// - LON: Longitude (WGS-84)
/// - GDA: Ground/Airborne status, A=Air, G=Ground
/// - SRC: Source of position, A=ADS-B, M=MLAT (always A in this case)
/// - ALT: Altitude/flight level
/// - SPD: Ground speed
/// - TRK: True track
/// - CAT: Category (A0-C7)
/// - SQU: Squawk
/// - VRT: Vertical rate
/// - MPS: MOPS
/// - NIC: NucP_NIC
///
#[serde_as]
#[derive(Clone, Debug, Deserialize, InfluxDbWriteable, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Avionix {
    /// UNIX timestamp in milli-secs (i64)
    #[serde(rename = "uti")]
    pub time: DateTime<Utc>,
    /// ESRI timestamp e.g. 2015-07-26 07:36:51.657189000
    pub dat: String,
    /// SIC
    pub sic: u8,
    /// SAC
    pub sac: u8,
    /// ICAO 6 byte code for the aircraft
    pub hex: String,
    /// Call-sign
    pub fli: String,
    /// Position latitude
    pub lat: f32,
    /// Position longitude
    pub lon: f32,
    /// Ground/Airborne status, A=Air, G=Ground
    pub gda: String,
    /// Source of position, A=ADS-B, M=MLAT (always A in this case)
    pub src: String,
    /// Altitude in feet
    pub alt: f32,
    /// Ground speed
    pub spd: f32,
    /// True track
    pub trk: f32,
    /// Category (A0 to C7)
    pub cat: String,
    /// Squawk
    pub squ: String,
    /// Vertical Rate
    pub vrt: f32,
    /// MOPS
    pub mps: u32,
    /// NucP_NIC
    pub nic: u32,
}

/// Special enum for airborne status
///
#[derive(Debug, Deserialize, Serialize, strum::Display, EnumString, EnumVariantNames)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Gda {
    /// Airborne
    A,
    /// Ground
    G,
}

/// Special enum for type of source, always ADS-B for Avionix
///
#[derive(Debug, Deserialize, Serialize, strum::Display, EnumString, EnumVariantNames)]
pub enum Src {
    /// ADS-B
    A,
    /// MLAT
    M,
}
