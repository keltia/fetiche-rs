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

use eyre::Result;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use strum::{EnumString, EnumVariantNames};
use tracing::debug;

pub use location::*;

use crate::{convert_to, to_feet, Bool, Cat21, TodCalculated, DEF_SAC, DEF_SIC};

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

#[derive(Debug, Deserialize, EnumString, strum::Display, EnumVariantNames)]
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

#[derive(Debug, Deserialize, strum::Display, EnumString, EnumVariantNames)]
#[strum(serialize_all = "UPPERCASE")]
pub enum AirGround {
    /// in Air
    A,
    /// On the Ground
    G,
}

/// A single position
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

convert_to!(from_flightaware, Position, Cat21);

impl From<&Position> for Cat21 {
    fn from(line: &Position) -> Self {
        let tod: i64 = line.clock as i64;
        let callsign = line.ident.clone();

        // WTF it can be < 0
        //
        let alt_baro_ft = match line.alt_gnss {
            Some(alt) => {
                if alt < 0 {
                    0
                } else {
                    alt
                }
            }
            None => 0,
        };
        Cat21 {
            sac: DEF_SAC,
            sic: DEF_SIC,
            alt_geo_ft: to_feet(line.alt.unwrap_or(0) as f32),
            pos_lat_deg: line.lat,
            pos_long_deg: line.lon,
            alt_baro_ft: alt_baro_ft as u32,
            tod: 128 * (tod % 86400),
            rec_time_posix: tod,
            rec_time_ms: 0,
            emitter_category: 13,
            differential_correction: Bool::N,
            ground_bit: Bool::N,
            simulated_target: Bool::N,
            test_target: Bool::N,
            from_ft: Bool::N,
            selected_alt_capability: Bool::N,
            spi: Bool::N,
            link_technology_cddi: Bool::N,
            link_technology_mds: Bool::N,
            link_technology_uat: Bool::N,
            link_technology_vdl: Bool::N,
            link_technology_other: Bool::N,
            descriptor_atp: 1,
            alt_reporting_capability_ft: 0,
            target_addr: 623615,
            cat: 21,
            line_id: 1,
            ds_id: 18,
            report_type: 3,
            tod_calculated: TodCalculated::N,
            callsign,
            groundspeed_kt: line.gs.unwrap_or(0) as f32,
            track_angle_deg: line.heading.unwrap_or(0.0),
            rec_num: 1,
        }
    }
}
