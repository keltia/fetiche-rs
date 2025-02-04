//! This module implement a subset of [FlightAware]'s [Firehose] API.
//!
//! Only the struct we need are there, this is not a general client API.
//!
//! NOTE: all fields are returned as `String` regardless of the actual data type.  This is I guess
//!       intentional by Flightaware to simplify internal stuff.  It is still a pain in the
//!       *ss to deal with.  `serde_with` is THE crate you want for this.
//!
//! Non-mandatory fields are `Option`.
//!
//! [FlightAware]: https://flightaware.com/
//! [Firehose]: https://flightaware.com/commercial/firehose/documentation/messages
//!

use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use strum::EnumString;

pub use location::*;

mod location;

#[derive(Debug, Deserialize, strum::Display, EnumString, strum::VariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum FeedType {
    Airborne,
    Surface,
    Weather,
}

#[derive(Clone, Debug, Deserialize, strum::Display, EnumString, strum::VariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum TimeType {
    Actual,
    EnRoute,
    Estimate,
}

/// Timestamps are in POSIX Epoch format (i32)
///
#[serde_as]
#[derive(Clone, Debug, Deserialize)]
pub struct Arrival {
    /// Arrival Time (i32)
    #[serde_as(as = "DisplayFromStr")]
    pub aat: i32,
    /// FlightAware flight id
    pub id: String,
    /// Flight identifier (callsign)
    pub ident: String,
    /// Point In Time Recovery (i32)
    #[serde_as(as = "DisplayFromStr")]
    pub pitr: i32,
    /// Arrival Time Type
    #[serde(rename = "timeType")]
    pub time_type: TimeType,
    /// Message Type: ALWAYS "arrival"
    #[serde(rename = "type")]
    pub atype: String,
    //
    /// ATC Ident
    pub atc_ident: Option<String>,
    /// Destination String, can be ICAO code, waypoint, or Lat/Lon pair
    pub dest: Option<String>,
    /// Estimated Departure Time (i32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub edt: Option<i32>,
    /// Estimated Time of Arrival (i32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub eta: Option<i32>,
    /// En route time (i32, in seconds)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub ete: Option<i32>,
    /// Reporting facility hash
    pub facility_hash: Option<String>,
    /// Reporting facility hash
    pub facility_name: Option<String>,
    /// Origin String, can be ICAO code, waypoint, or Lat/Lon pair
    pub orig: Option<String>,
    /// Aircraft Registration
    pub reg: Option<String>,
    /// Synthetic flag (bool, "1" == true)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub synthetic: Option<u8>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Cancellation {
    /// FlightAware flight id
    pub id: String,
    /// Flight identifier (callsign)
    pub ident: String,
    /// Origin String, can be ICAO code, waypoint, or Lat/Lon pair
    pub orig: Option<String>,
    /// Point In Time Recovery (i32)
    #[serde_as(as = "DisplayFromStr")]
    pub pitr: i32,
    /// Message Type: ALWAYS "cancellation"
    #[serde(rename = "type")]
    pub ctype: String,
    //
    /// Aircraft Type
    pub aircraft_type: Option<String>,
    /// Filed cruising alt (u32, in feet — network order MSL)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub alt: Option<u32>,
    /// ATC Ident
    pub atc_ident: Option<String>,
    /// Destination String, can be ICAO code, waypoint, or Lat/Lon pair -- see `Location`
    pub dest: Option<String>,
    /// Estimated Departure Time (i32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub edt: Option<i32>,
    /// Estimated Time of Arrival (i32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub eta: Option<i32>,
    /// En route time (in seconds) (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub ete: Option<u32>,
    /// Reporting facility hash
    pub facility_hash: Option<String>,
    /// Reporting facility hash
    pub facility_name: Option<String>,
    /// Filed departure time (i32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub fdt: Option<i32>,
}

#[derive(Debug)]
pub struct Departure {}

#[derive(Debug)]
pub struct ExtendedFlightInfo {}

#[derive(Debug, Deserialize)]
pub struct Error {
    /// Error Message
    pub error_msg: String,
    /// Message Type ("error")
    #[serde(rename = "type")]
    pub etype: String,
}

#[derive(Debug)]
pub struct Flifo {}

#[derive(Debug)]
pub struct Flightplan {}

#[derive(Debug)]
pub struct Fmswx {}

#[derive(Debug)]
pub struct GroundPosition {}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Keepalive {
    /// PITR resume value
    #[serde_as(as = "DisplayFromStr")]
    pub pitr: i32,
    /// Time of keepalive generation
    #[serde(rename = "serverTime")]
    #[serde_as(as = "DisplayFromStr")]
    pub server_time: i32,
    /// Message Type ("keepalive")
    #[serde(rename = "type")]
    pub ktype: String,
}

#[derive(Debug)]
pub struct LocationEntry {}

#[derive(Debug)]
pub struct LocationExit {}

#[derive(Debug)]
pub struct PowerOn {}

#[derive(Debug, Deserialize, EnumString, strum::Display, strum::VariantNames)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Update {
    /// ADS-B
    A,
    /// Radar
    Z,
    /// Transoceanic
    O,
    ///Estimated
    P,
    ///Datalink
    D,
    ///MLAT
    M,
    /// ADSE-X
    X,
    ///Space-based ADS-B
    S,
}

#[derive(Debug, Deserialize, strum::Display, EnumString, strum::VariantNames)]
#[strum(serialize_all = "UPPERCASE")]
pub enum AirGround {
    /// in Air
    A,
    /// On the Ground
    G,
}

/// Represents a single position report in the FlightAware system.
///
/// A position report provides detailed information about a specific location,
/// including geographical latitude and longitude, the air or ground status,
/// and other related data.
///
/// # Fields
///
/// - `air_ground` (`AirGround`):
///   Specifies whether the position report is for an aircraft in the air or on the ground.
///
/// - `clock` (`i32`):
///   The report time represented as a UNIX Epoch timestamp.
///
/// - `facility_hash` (`String`):
///   The unique hash of the reporting facility.
///
/// - `facility_name` (`String`):
///   The name of the reporting facility.
///
/// - `id` (`String`):
///   A unique identifier for the report.
///
/// - `ident` (`String`):
///   The flight identifier or callsign associated with the report.
///
/// - `lat` (`f32`):
///   The geographical latitude of the position.
///
/// - `lon` (`f32`):
///   The geographical longitude of the position.
///
/// - `pitr` (`i32`):
///   Value used for Point In Time Recovery.
///
/// - `ptype` (`String`):
///   The type of message associated with the position.
///
/// - `update_type` (`Update`):
///   The type of update this position represents (e.g., ADS-B, Radar).
///
/// - `adsb_version` (`Option<String>`):
///   The ADS-B version used for the report, if available.
///
/// - `aircraft_type` (`Option<String>`):
///   The ICAO aircraft type code, if available.
///
/// - `alt` (`Option<i32>`):
///   The altitude in feet, if available.
///
/// - `alt_gnss` (`Option<i32>`):
///   The GNSS altitude in feet over WGS84, if available.
///
/// - `alt_change` (`Option<String>`):
///   Specifies the change in altitude (e.g., "C" for climbing or "D" for descending).
///
/// - `atc_ident` (`Option<String>`):
///   The ATC identifier, if available.
///
/// - `dest` (`Option<String>`):
///   The destination, which can be expressed as an ICAO code, waypoint, or geographical coordinates.
///
/// - `edt` (`Option<i32>`):
///   The estimated departure time as a UNIX Epoch timestamp.
///
/// - `eta` (`Option<i32>`):
///   The estimated time of arrival as a UNIX Epoch timestamp.
///
/// - `ete` (`Option<i32>`):
///   The estimated en route time in seconds.
///
/// - `gs` (`Option<u32>`):
///   The ground speed in knots, if available.
///
/// - `heading` (`Option<f32>`):
///   The heading (in degrees) relative to either magnetic or true North, depending on other fields.
///
/// - `heading_magnetic` (`Option<f32>`):
///   The heading relative to magnetic North in degrees.
///
/// - `heading_true` (`Option<f32>`):
///   The heading relative to true North in degrees.
///
/// - `hexid` (`Option<String>`):
///   The Mode S transponder code, if available.
///
/// - `mach` (`Option<String>`):
///   The Mach number of the aircraft, if available.
///
/// - `nac_p` (`Option<u32>`):
///   The Navigational Accuracy Category for Position (NACp) in ADS-B.
///
/// - `nac_v` (`Option<u32>`):
///   The Navigational Accuracy Category for Velocity (NACv) in ADS-B.
///
/// - `nav_altitude` (`Option<f32>`):
///   The altitude used in the navigation system in feet.
///
/// - `nav_heading` (`Option<f32>`):
///   The navigation heading (degrees).
///
/// - `nav_modes` (`Option<String>`):
///   A comma-separated list of active navigation modes (e.g., autopilot, VNAV).
///
/// - `nav_qnh` (`Option<f32>`):
///   The altimeter setting used for navigation purposes in hPa (hectopascals).
///
/// - `nic` (`Option<u32>`):
///   The Navigation Integrity Category (NIC) used in ADS-B systems.
///
/// - `nic_baro` (`Option<u32>`):
///   The Navigation Integrity Category for barometric data in ADS-B.
///
/// - `orig` (`Option<String>`):
///   The origin of the flight, which can be expressed as an ICAO code, waypoint, or geographical coordinates.
///
/// - `dest` (`Option<String>`):
///   The destination of the flight, which can also be ICAO, waypoint, or lat/lon.
///
#[serde_as]
#[derive(Debug, Deserialize)]
struct Position {
    /// Air/Ground
    #[serde_as(as = "DisplayFromStr")]
    pub air_ground: AirGround,
    /// Report time (UNIX Epoch) (i32)
    #[serde_as(as = "DisplayFromStr")]
    pub clock: i32,
    /// Reporting facility hash
    pub facility_hash: String,
    /// Reporting facility hash
    pub facility_name: String,
    /// FlightAware flight id
    pub id: String,
    /// Flight identifier (callsign)
    pub ident: String,
    /// Latitude
    #[serde_as(as = "DisplayFromStr")]
    pub lat: f32,
    /// Longitude
    #[serde_as(as = "DisplayFromStr")]
    pub lon: f32,
    /// Point In Time Recovery (i32)
    #[serde_as(as = "DisplayFromStr")]
    pub pitr: i32,
    /// Message type
    #[serde(rename = "type")]
    pub ptype: String,
    /// Update Type
    #[serde(rename = "updateType")]
    #[serde_as(as = "DisplayFromStr")]
    pub update_type: Update,
    //
    /// ADS-B version
    pub adsb_version: Option<String>,
    /// ICAO Aircraft Type Code
    #[serde(rename = "aircrafttype")]
    pub aircraft_type: Option<String>,
    /// Altitude
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub alt: Option<i32>,
    /// GNSS Altitude (feet over WGS84)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub alt_gnss: Option<i32>,
    /// Altitude Change ("C", "D" or " ")
    #[serde(rename = "altChange")]
    pub alt_change: Option<String>,
    /// ATC Ident
    pub atc_ident: Option<String>,
    /// Destination String, can be ICAO code, waypoint, or Lat/Lon pair
    pub dest: Option<String>,
    /// Estimated Departure Time (i32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub edt: Option<i32>,
    /// Estimated Time of Arrival (i32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub eta: Option<i32>,
    /// En route time (i32, in seconds)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub ete: Option<i32>,
    /// Ground Speed (knots) (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub gs: Option<u32>,
    /// Course (degrees) (f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub heading: Option<f32>,
    /// Heading relative to magnetic North (degrees) (f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub heading_magnetic: Option<f32>,
    /// Heading relative to true North (degrees) (f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub heading_true: Option<f32>,
    /// Transponder Mode S code
    pub hexid: Option<String>,
    /// Mach Number
    pub mach: Option<String>,
    /// NACp (ADS-B Navigational Accuracy Category for Position)
    pub nac_p: Option<u32>,
    /// NACv (ADS-B Navigational Accuracy Category for Velocity)
    pub nac_v: Option<u32>,
    /// Navigation Altitude (f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub nav_altitude: Option<f32>,
    /// Navigation Heading (degrees) (f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub nav_heading: Option<f32>,
    /// Navigation modes (autopilot, vnav, althold, approach, lnav, tcas)
    pub nav_modes: Option<String>,
    /// Navigation Altimeter Settings (f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub nav_qnh: Option<f32>,
    /// ADS-B Navigation Integrity Category
    pub nic: Option<u32>,
    /// ADS-B Navigation Integrity Category for barometer
    pub nic_baro: Option<u32>,
    /// Origin (actually a Location)
    pub orig: Option<String>,
    /// Radius of Containment (m) (u32)
    pub pos_rc: Option<u32>,
    /// Aircraft Registration
    pub reg: Option<String>,
    /// Textual Route string
    pub route: Option<String>,
    /// ADS-B Source Integrity Level
    pub sil: Option<u32>,
    /// SIL type (per-hour or per-sample)
    pub sil_type: Option<String>,
    /// Filed cruising speed (knots) (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub speed: Option<u32>,
    /// Indicated Air Speed (knots) (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub speed_ias: Option<u32>,
    /// True Air Speed (knots) (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub speed_tas: Option<u32>,
    /// Transponder Squawk code
    pub squawk: Option<String>,
    /// Computed Outside Air Temp. (f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub temperature: Option<f32>,
    /// Quality (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub temperature_quality: Option<u32>,
    /// Vertical Rate (feet/mn) (u32)
    #[serde(rename = "vertRate")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub vert_rate: Option<i32>,
    /// Geometric Vertical Rate — GNSS (feet/mn) (u32)
    #[serde(rename = "vertRate_geom")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub vert_rate_geom: Option<i32>,
    /// List of 2D/3D/4D objects of locations
    pub waypoints: Option<Vec<String>>,
    /// Computed Wind Direction (f32) 0 = from North
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub wind_dir: Option<f32>,
    /// 1 is aircraft is stable, 0 otherwise (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub wind_quality: Option<i8>,
    /// Computed Wind Speed (knots) (u32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub wind_speed: Option<u32>,
}
