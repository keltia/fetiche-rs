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
            Some(v) => v.iter().map(|s| Cat21::from(s)).collect(),
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
