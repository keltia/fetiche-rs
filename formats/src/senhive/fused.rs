//! This is the module for the data types for the `fused_data` / `dl_fused_data` queues.
//!

// ----- queue: `fused_data`

use crate::senhive::Coordinates;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use strum::EnumString;

#[derive(Debug, Serialize, Deserialize)]
pub struct Location1 {
    pub coordinates: Coordinates,
    pub uncertainty: Option<f64>,
    /// This is a string with a 7-point WKT Polygon
    pub likelihood: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PilotState {
    pub location: Location1,
    #[serde(rename = "locationType")]
    pub location_type: i64,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Altitudes {
    pub ato: FusedValue,
    pub agl: FusedValue,
    pub amsl: Option<FusedValue>,
    pub geodetic: FusedValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub coordinates: Coordinates,
    pub uncertainty: Option<f64>,
    /// This is a string with a 7-point WKT Polygon
    pub likelihood: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VehicleState {
    pub location: Location,
    pub altitudes: Altitudes,
    #[serde(rename = "groundSpeed")]
    pub ground_speed: Option<FusedValue>,
    #[serde(rename = "verticalSpeed")]
    pub vertical_speed: Option<f64>,
    pub orientation: Option<FusedValue>,
    pub state: i64,
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
