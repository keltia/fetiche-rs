//! This module implement a subset of [FlightAware]'s [Firehose] API.
//!
//! Only the struct we need are there, this is not a general client API.
//!
//! NOTE: all fields are `String` regardless of the actual data type.  This is I guess
//!       intentional by Flightaware to simplify internal stuff.  It is still a pain in the
//!       *ss to deal with.  Not sure if I want to have two sets of types, the string and the real
//!       ones to work with.
//!
//! Non-mandatory fields are `Option`.
//!
//! [FlightAware]: https://flightaware.com/
//! [Firehose]: https://flightaware.com/commercial/firehose/documentation/messages
//!

use serde::Deserialize;
use strum::{EnumString, EnumVariantNames};

pub use location::*;

mod location;

#[derive(Debug, Deserialize, strum::Display, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum FeedType {
    Airborne,
    Surface,
    Weather,
}

#[derive(Clone, Debug, Deserialize, strum::Display, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum TimeType {
    Actual,
    EnRoute,
    Estimate,
}

/// Timestamps are in POSIX Epoch format (i32)
///
#[derive(Clone, Debug, Deserialize)]
pub struct Arrival {
    /// Arrival Time (i32)
    pub aat: String,
    /// FlightAware flight id
    pub id: String,
    /// Flight identifier (callsign)
    pub ident: String,
    /// Point In Time Recovery (i32)
    pub pitr: String,
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
    pub edt: Option<String>,
    /// Estimated Time of Arrival (i32)
    pub eta: Option<String>,
    /// En route time (i32, in seconds)
    pub ete: Option<String>,
    /// Reporting facility hash
    pub facility_hash: Option<String>,
    /// Reporting facility hash
    pub facility_name: Option<String>,
    /// Origin String, can be ICAO code, waypoint, or Lat/Lon pair
    pub orig: Option<String>,
    /// Aircraft Registration
    pub reg: Option<String>,
    /// Synthetic flag (bool, "1" == true)
    pub synthetic: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Cancellation {
    /// FlightAware flight id
    pub id: String,
    /// Flight identifier (callsign)
    pub ident: String,
    /// Origin String, can be ICAO code, waypoint, or Lat/Lon pair
    pub orig: Option<String>,
    /// Point In Time Recovery (i32)
    pub pitr: String,
    /// Message Type: ALWAYS "cancellation"
    #[serde(rename = "type")]
    pub ctype: String,
    //
    /// Aircraft Type
    pub aircraft_type: Option<String>,
    /// Filed cruising alt (u32, in feet — network order MSL)
    pub alt: Option<String>,
    /// ATC Ident
    pub atc_ident: Option<String>,
    /// Destination String, can be ICAO code, waypoint, or Lat/Lon pair -- see `Location`
    pub dest: Option<String>,
    /// Estimated Departure Time (i32)
    pub edt: Option<String>,
    /// Estimated Time of Arrival (i32)
    pub eta: Option<String>,
    /// En route time (in seconds) (i32)
    pub ete: Option<String>,
    /// Reporting facility hash
    pub facility_hash: Option<String>,
    /// Reporting facility hash
    pub facility_name: Option<String>,
    /// Filed departure time (i32)
    pub fdt: Option<String>,
}

#[derive(Debug)]
pub struct Departure {}

#[derive(Debug)]
pub struct ExtendedFlightInfo {}

#[derive(Debug)]
pub struct Error {}

#[derive(Debug)]
pub struct Flifo {}

#[derive(Debug)]
pub struct Flightplan {}

#[derive(Debug)]
pub struct Fmswx {}

#[derive(Debug)]
pub struct GroundPosition {}

#[derive(Debug)]
pub struct LocationEntry {}

#[derive(Debug)]
pub struct LocationExit {}

#[derive(Debug)]
pub struct PowerOn {}

#[derive(Debug)]
struct Position {}