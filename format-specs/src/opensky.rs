//! Module to load and process the data coming from the OpenskyD site and generate
//! CSV data Cat21-like
//!
//! XXX they send out an array of arrays, each representing a specific state vector.
//!     it sucks.
//!
//! XXX Due to this, I'm not sure converting these state vectors into our Cat21 makes any sense.
//!
//! Documentation is taken from [The Opensky site](https://opensky-network.github.io/opensky-api/rest.html)
//!

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_repr::Deserialize_repr;

use crate::{to_feet, to_knots, Bool, Cat21, TodCalculated};

/// Origin of state's position
///
#[derive(Debug, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum Source {
    AdsB = 0,
    Asterix,
    MLAT,
    FLARM,
}

/// Aircraft category
///
#[derive(Debug, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum Category {
    NoInfo = 0,
    NoAdsBEmitterCategoryInfo,
    Light,
    Small,
    Large,
    HighVortexLarge,
    Heavy,
    HighPerformance,
    RotorCraft,
    Glider,
    Lighter,
    Skydiver,
    UltraLight,
    Reserved,
    Space,
    SurfaceEmergencyVehicule,
    SurfaceServiceVehicule,
    PointObstacle,
    ClusterObstacle,
    LineObstacle,
}

#[derive(Debug, Deserialize)]
pub struct Opensky {
    /// UNIX timestamps
    pub time: i32,
    /// The state vectors
    pub states: Option<Vec<StateVector>>,
}

impl Opensky {
    /// Transform a given record into an array of Cat21 records
    ///
    pub fn to_cat21(&self) -> Vec<Cat21> {
        match &self.states {
            Some(v) => v.iter().map(Cat21::from).collect(),
            None => vec![],
        }
    }

    /// Deserialize from json
    ///
    pub fn from_json(input: &str) -> Result<Opensky> {
        let data: Opensky = serde_json::from_str(input)?;
        Ok(data)
    }
}

/// Definition of a state vector as generated
///
#[derive(Debug, Deserialize)]
pub struct StateVector {
    /// ICAO ID
    pub icao24: String,
    pub callsign: Option<String>,
    pub origin_country: String,
    pub time_position: Option<i32>,
    pub last_contact: i32,
    /// Position
    pub longitude: Option<f32>,
    pub latitude: Option<f32>,
    pub baro_altitude: Option<u32>,
    pub on_ground: bool,
    pub velocity: Option<i32>,
    pub true_track: Option<f32>,
    pub vertical_rate: Option<f32>,
    pub sensors: Option<Vec<u32>>,
    pub geo_altitude: Option<f32>,
    pub squawk: Option<String>,
    pub spi: bool,
    /// Position source
    pub position_source: Source,
    /// Aircraft category
    pub category: Category,
}

impl From<&StateVector> for Cat21 {
    fn from(line: &StateVector) -> Self {
        let tp = format!("{}", line.time_position.unwrap_or(0));
        let tod = tp.parse::<DateTime<Utc>>().unwrap();
        let tod = tod.timestamp();
        let callsign = line.callsign.clone().unwrap_or("".to_string());

        Cat21 {
            sac: 8,
            sic: 200,
            alt_geo_ft: to_feet(line.geo_altitude.unwrap_or(0.0)),
            pos_lat_deg: line.latitude.unwrap_or(0.0),
            pos_long_deg: line.longitude.unwrap_or(0.0),
            alt_baro_ft: to_feet(line.baro_altitude.unwrap_or(0) as f32),
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
            groundspeed_kt: to_knots(line.velocity.unwrap_or(0) as f32),
            track_angle_deg: line.true_track.unwrap_or(0.0),
            rec_num: 1,
        }
    }
}
