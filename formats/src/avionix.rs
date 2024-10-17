//! This is the pseudo-cat21 file format specified in the Avionix documentation.
//!
//! URL: http://www.avionix.pl
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::EnumString;

/// Avionix CUBE drone antenna output format
///
/// This is used in the [Aero Network API](https://aero-network.com/api) for drone data..
///
/// Payload is in JSON.
///
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AvionixServer {
    #[serde(rename = "uti")]
    /// - uti   Timestamp of last message, seconds since 1.1.1970 00:00 UTC -- Integer -- 1576153180
    pub time: u32,
    /// - dat   UTC timestamp of message, time in nanosecond resolution -- String -- “2019-12-12 12:19:40.291276211”
    pub dat: String,
    /// - hex   ICAO 24-bit Hex transponder ID -- String -- “44ce6f”
    pub hex: String,
    /// - tim   Timestamp of last received message, nanosecond resolution -- String -- “12:19:40.29127621”
    pub tim: String,
    /// - fli   Flight Identification/Call Sign -- String -- “EWG3ZX”
    pub fli: String,
    /// - lat   Latitude (WGS-84) in decimal degrees -- Float -- 50.902073
    pub lat: f64,
    /// - lon   Longitude (WGS-84) in decimal degrees -- Float -- 2.4822274
    pub lon: f64,
    /// - gda   Ground/Air status A=Air G=GND -- String -- “G”
    pub gda: String,
    /// - src   Source of position -- See  `Src`
    pub src: String,
    /// - alt   Altitude in feet 1013 hPa Standard Atmosphere -- Integer -- 5440
    pub alt: u32,
    /// - altg  Geometric altitude in feet -- Integer -- 5400
    pub altg: u32,
    /// - hgt   Difference between barometric and geometric altitude in ft* -- Integer -- -225
    pub hgt: Option<i32>,
    /// - spd   Ground speed in knots -- Integer -- 49
    pub spd: u32,
    /// - cat   Empty if not known, or A0-C7 for ADS-B/MLAT/Remote-ID or
    ///         O1-O15 for data on SRD860 (see `Category`) -- String -- “A0”
    pub cat: String,
    /// - squ   Squawk SSR Mode A code (4 digit octal) -- String -- “5763”
    pub squ: String,
    /// - vrt   Vertical Rate in ft/min -- Integer -- -128
    pub vrt: i32,
    /// - trk   True track in degrees -- Float -- 154.5
    pub trk: f64,
    /// - mop   Operational performance (0=DO260, 1=DO260A, 2=DO260B) -- Integer -- 0
    pub mop: u32,
    /// - lla   Age of last position update, in seconds -- Integer -- 0
    pub lla: u32,
    /// - tru   Number of packets received for tracked flight -- Integer -- 213
    pub tru: usize,
    /// - dbm   Signal strentgh of last received message -- Integer -- -91
    pub dbm: i32,
    /// - shd   Selected heading* -- Integer -- 293
    pub shd: Option<u32>,
    /// - org   ICAO code airport of origin* -- String “EDDK”
    pub org: Option<String>,
    /// - dst   ICAO code airport of destination* -- String -- “EPKK”
    pub dst: Option<String>,
    /// - opr   Operator* -- String -- “GWI”
    pub opr: Option<String>,
    /// - typ   Aircraft type* -- String “A319”
    pub typ: Option<String>,
    /// - reg   Registration* -- String “D-AKNM”
    pub reg: Option<String>,
    /// - cou   Country* -- String -- “Germany”
    pub cou: Option<String>,
}

/// Avionix CUBE drone antenna output format
///
/// This is used when connecting to the antenna directly through selected port.
/// Port is 50005/tcp for the json payload.
///
/// This effectively group all sources into one stream:
/// - 1090 MHz for ADS-B
/// - 868 MHz for OGN/FLARM/ADS-L
/// - 2.4 GHz for Remote-ID
///
/// Payload is in JSON.
///
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AvionixCube {
    #[serde(rename = "uti")]
    /// - uti   Timestamp of last message, seconds since 1.1.1970 00:00 UTC -- Integer -- 1576153180
    pub time: u32,
    /// - dat   UTC timestamp of message, time in nanosecond resolution -- String -- “2019-12-12 12:19:40.291276211”
    pub dat: String,
    /// - hex   ICAO 24-bit Hex transponder ID -- String -- “44ce6f”
    pub hex: String,
    /// - tim   Timestamp of last received message, nanosecond resolution -- String -- “12:19:40.29127621”
    pub tim: String,
    /// - fli   Flight Identification/Call Sign -- String -- “EWG3ZX”
    pub fli: String,
    /// - lat   Latitude (WGS-84) in decimal degrees -- Float -- 50.902073
    pub lat: f64,
    /// - lon   Longitude (WGS-84) in decimal degrees -- Float -- 2.4822274
    pub lon: f64,
    /// - gda   Ground/Air status A=Air G=GND -- String -- “G”
    pub gda: String,
    /// - src   Source of position -- See  `Src`
    pub src: String,
    /// - alt   Altitude in feet 1013 hPa Standard Atmosphere -- Integer -- 5440
    pub alt: u32,
    /// - spd   Ground speed in knots -- Integer -- 49
    pub spd: u32,
    /// - trk   True track in degrees -- Integer -- 154
    pub trk: i32,
    /// - cat   Empty if not known, or A0-C7 for ADS-B/MLAT/Remote-ID or
    ///         O1-O15 for data on SRD860 (see `Category`) -- String -- “A0”
    pub cat: String,
    /// - hgt   Difference between barometric and geometric altitude in ft* -- Integer -- -225
    pub hgt: Option<i32>,
    /// - shd   Selected heading* -- Integer -- 293
    pub shd: Option<u32>,
    /// - org   ICAO code airport of origin* -- String “EDDK”
    pub org: Option<String>,
    /// - dst   ICAO code airport of destination* -- String -- “EPKK”
    pub dst: Option<String>,
    /// - opr   Operator* -- String -- “GWI”
    pub opr: Option<String>,
    /// - typ   Aircraft type* -- String “A319”
    pub typ: Option<String>,
    /// - reg   Registration* -- String “D-AKNM”
    pub reg: Option<String>,
    /// - squ   Squawk SSR Mode A code (4 digit octal) -- String -- “5763”
    pub squ: String,
    /// - dis   Distance from receiver in km -- Float --
    pub dis: f64,
    /// - cou   Country* -- String -- “Germany”
    pub cou: Option<String>,
    /// - mop   Operational performance (0=DO260, 1=DO260A, 2=DO260B) -- Integer -- 0
    pub mop: Option<u32>,
    /// - tru   Number of packets received for tracked flight -- Integer -- 213
    pub tru: usize,
    /// - vrt   Vertical Rate in ft/min -- Integer -- -128239.25
    pub vrt: i32,
    /// - lla   Age of last position update, in seconds -- Integer -- 0
    pub lla: u32,
}

// -----

/// Avionix pseudo-Cat21 coming from the ADS-B receiver.
///
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
/// ** DEPRECATED **
///
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AvionixCat21 {
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
#[derive(Debug, Deserialize, Serialize, strum::Display, EnumString, strum::VariantNames)]
#[strum(serialize_all = "UPPERCASE")]
enum Gda {
    /// Airborne
    A,
    /// Ground
    G,
}

/// Special enum for type of source, always ADS-B for Avionix
///
#[derive(Debug, Deserialize, Serialize, strum::Display, EnumString, strum::VariantNames)]
enum Src {
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
    RID,
    /// 4G/5G
    LTE,
    /// PilotAware
    P,
    /// FANET
    N,
    /// Asterix
    X,
}

/// Object type
///
#[derive(Debug, Deserialize, Serialize, strum::Display, EnumString, strum::VariantNames)]
enum Category {
    /// Unknown
    O0,
    /// Glider
    O1,
    /// Tow Plane
    O2,
    /// Helicopter or Rotorcraft
    O3,
    /// Parachute
    O4,
    /// Drop Plane
    O5,
    /// Hand Glider
    O6,
    /// Para Glider
    O7,
    /// Powered Aircraft
    O8,
    /// Jet Aircraft
    O9,
    /// UFO (lol)
    O10,
    /// Balloon
    O11,
    /// Airship
    O12,
    /// UAV
    O13,
    /// Ground Vehicule
    O14,
}
