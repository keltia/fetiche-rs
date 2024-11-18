//! This is the module for the data types for the `fused_data` / `dl_fused_data` queues.
//!

// ----- queue: `fused_data`

use crate::senhive::Coordinates;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::EnumString;

// ----- Original raw data format

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub coordinates: Coordinates,
    pub uncertainty: Option<f64>,
    /// This is a string with a 7-point WKT Polygon
    pub likelihood: Option<String>,
}
/// "Measurement type as Integer. Can be 'Take-off location' (0), 'UAV Home location' (1), 'Live measurement update' (2), or 'unknown' (15)"
#[derive(Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum LocationType {
    TakeOff = 0,
    Home = 1,
    Live = 2,
    #[default]
    Unknown = 15,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PilotState {
    pub location: Location,
    #[serde(rename = "locationType")]
    pub location_type: LocationType,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PilotIdentification {
    #[serde(rename = "operatorID")]
    pub operator_id: String,
    pub other: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FusedValue {
    pub value: f64,
    pub uncertainty: Option<f64>,
}

impl From<FusedValue> for f64 {
    /// Easy conversion into plain f64
    fn from(fv: FusedValue) -> Self {
        fv.value
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Altitudes {
    /// Above take-off location [m]
    pub ato: Option<FusedValue>,
    /// Above ground level [m]
    pub agl: Option<FusedValue>,
    /// Above mean sea level [m]
    pub amsl: Option<FusedValue>,
    /// Real geodetic altitude.
    pub geodetic: Option<FusedValue>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum VehicleStateType {
    MotorOff = 0,
    MotorOn = 1,
    Airborn = 2,
    #[default]
    Unknown = 15,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VehicleState {
    pub location: Location,
    pub altitudes: Altitudes,
    #[serde(rename = "groundSpeed")]
    pub ground_speed: Option<FusedValue>,
    #[serde(rename = "verticalSpeed")]
    pub vertical_speed: Option<FusedValue>,
    pub orientation: Option<FusedValue>,
    pub state: Option<i8>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct VehicleIdentification {
    pub serial: Option<String>,
    pub mac: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "uavType")]
    pub uav_type: u8,
}

#[derive(Debug, Deserialize, strum::Display, EnumString, strum::VariantNames, Serialize)]
#[repr(u8)]
pub enum FusionType {
    Cooperative = 0,
    Surveillance = 1,
    Both = 2,
}

#[derive(
    Debug,
    Default,
    Deserialize,
    strum::Display,
    EnumString,
    strum::VariantNames,
    Serialize
)]
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

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct FusionState {
    #[serde(rename = "fusionType")]
    pub fusion_type: u8,
    #[serde(rename = "sourceSerials")]
    pub source_serials: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct System {
    #[serde(rename = "trackID")]
    pub track_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "timestampLog")]
    pub timestamp_log: Option<Vec<TSLog>>,
    #[serde(rename = "fusionState")]
    pub fusion_state: FusionState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TSLog {
    pub process_name: String,
    pub timestamp: DateTime<Utc>,
    pub msg: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FusedData {
    pub version: String,
    pub system: System,
    #[serde(rename = "vehicleIdentification")]
    pub vehicle_identification: VehicleIdentification,
    #[serde(rename = "vehicleState")]
    pub vehicle_state: VehicleState,
    #[serde(rename = "pilotIdentification")]
    pub pilot_identification: Option<PilotIdentification>,
    #[serde(rename = "pilotState")]
    pub pilot_state: PilotState,
}

// ----- New `DronePoint`, flattened struct

/// This is a flattened struct representing items of value from the JSON record.
/// It mimics the `[Asd`](../asd.rs) struct.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct DronePoint {
    /// timestamp -- system.timestamp
    pub time: DateTime<Utc>,
    /// Each record is part of a drone journey with a specific ID -- system.track_id
    pub journey: String,
    /// Identifier for the drone -- vehicle_identification.serial
    pub ident: Option<String>,
    /// Maker of the drone -- vehicle_identification.make
    pub make: Option<String>,
    /// Model of the drone -- vehicle_identification.model
    pub model: Option<String>,
    /// UAV Type -- vehicle_identification.uav_type
    pub uav_type: UAVType,
    /// Source -- system.fusion_state.fusion_type
    pub source: FusionType,
    /// Latitude -- vehicle_state.location.coordinates.lat
    pub latitude: f32,
    /// Longitude -- vehicle_state.location.coordinates.lon
    pub longitude: f32,
    /// Altitude -- vehicle_state.altitudes.geodetic
    pub altitude: Option<u32>,
    /// Distance to ground -- vehicle_state.altitudes.ato.value
    pub elevation: Option<u32>,
    /// Operator lat -- pilot_state.location.coordinates.lat
    pub home_lat: Option<f32>,
    /// Operator lon -- pilot_state.location.coordinates.lon
    pub home_lon: Option<f32>,
    /// Altitude from takeoff point -- (vehicle_state.altitudes.ato.value - )
    pub home_height: Option<f32>,
    /// Current speed -- vehicle_state.ground_speed
    pub speed: f32,
    /// True heading -- vehicle_state.orientation
    pub heading: f32,
    /// Vehicle state -- vehicle_state.state
    pub state: Option<VehicleStateType>,
    /// Name of detecting point -- system.fusion_state.source_serials
    pub station_name: Option<String>,
    /// Latitude -- site latitude
    pub station_latitude: Option<f32>,
    /// Longitude -- site longitude
    pub station_longitude: Option<f32>,
}
