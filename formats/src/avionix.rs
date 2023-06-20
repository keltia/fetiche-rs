//! This is the pseudo-cat21 file format specified in the Avionix documentation.
//!
//! URL: http://www.avionix.pl
//!

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::ICAOString;

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
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Avionix {
    pub uti: u64,
    pub dat: String,
    pub sic: usize,
    pub sac: usize,
    pub hex: ICAOString,
    pub fli: String,
    pub lat: f32,
    pub lon: f32,
    pub gda: Gda,
    pub src: Src,
    pub alt: f32,
    pub spd: f32,
    pub trk: f32,
    pub cat: String,
    pub squ: String,
    pub vrt: f32,
    pub mps: usize,
    pub nic: usize,
}

/// Special enum for airborne status
///
#[derive(Debug, Deserialize, Serialize)]
pub enum Gda {
    A,
    G,
}

impl Display for Gda {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            Gda::A => "A",
            Gda::G => "G",
        };
        write!(f, "{}", r)
    }
}

impl From<&str> for Gda {
    fn from(value: &str) -> Self {
        match value {
            "A" => Gda::A,
            "G" => Gda::G,
            _ => Gda::A,
        }
    }
}

/// Special enum for type of source, always ADS-B for Avionix
///
#[derive(Debug, Deserialize, Serialize)]
pub enum Src {
    A,
    M,
}

impl Display for Src {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let r = match self {
            Src::A => "ADSB",
            Src::M => "MLAT",
        };
        write!(f, "{}", r)
    }
}

impl From<&str> for Src {
    fn from(value: &str) -> Self {
        match value {
            "A" => Src::A,
            "M" => Src::M,
            _ => Src::A,
        }
    }
}
