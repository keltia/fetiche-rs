// ----- New `DronePoint`, flattened struct

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;

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
pub enum DataSource {
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

impl From<crate::DataSource> for u8 {
    fn from(value: crate::DataSource) -> Self {
        match value {
            crate::DataSource::A => 0,
            crate::DataSource::M => 1,
            crate::DataSource::U => 2,
            crate::DataSource::L => 3,
            crate::DataSource::F => 4,
            crate::DataSource::O => 5,
            crate::DataSource::Rid => 6,
            crate::DataSource::Lte => 7,
            crate::DataSource::P => 8,
            crate::DataSource::N => 9,
            crate::DataSource::X => 10,
        }
    }
}

impl crate::DataSource {
    /// Direct mapping between a string and the u8 value as a source.
    ///
    pub fn str_to_source(value: &str) -> u8 {
        match value {
            "A" => 0,
            "M" => 1,
            "U" => 2,
            "L" => 3,
            "F" => 4,
            "O" => 5,
            "Rid" => 6,
            "Lte" => 7,
            "P" => 8,
            "N" => 9,
            "X" => 10,
            _ => 255,
        }
    }
}


