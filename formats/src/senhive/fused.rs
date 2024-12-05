//! This is the module for the data types for the `fused_data` / `dl_fused_data` queues.
//!

// ----- queue: `fused_data`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::senhive::Coordinates;
use crate::DronePoint;

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
    pub location_type: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PilotIdentification {
    #[serde(rename = "operatorID")]
    pub operator_id: String,
    pub other: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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

impl Default for FusedValue {
    fn default() -> Self {
        Self {
            value: 0.0,
            uncertainty: None,
        }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct VehicleState {
    pub location: Location,
    pub altitudes: Altitudes,
    #[serde(rename = "groundSpeed")]
    pub ground_speed: Option<FusedValue>,
    #[serde(rename = "verticalSpeed")]
    pub vertical_speed: Option<FusedValue>,
    pub orientation: Option<FusedValue>,
    pub state: Option<u8>,
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

/// Now define the mapping between our type `FusedData` and `DronePoint`:
///
///     /// timestamp -- system.timestamp
///     pub time: DateTime<Utc>,
///     /// Each record is part of a drone journey with a specific ID -- system.track_id
///     pub journey: String,
///     /// Identifier for the drone -- vehicle_identification.serial
///     pub ident: Option<String>,
///     /// Maker of the drone -- vehicle_identification.make
///     pub make: Option<String>,
///     /// Model of the drone -- vehicle_identification.model
///     pub model: Option<String>,
///     /// UAV Type -- vehicle_identification.uav_type
///     pub uav_type: u8,
///     /// Source -- system.fusion_state.fusion_type
///     pub source: u8,
///     /// Latitude -- vehicle_state.location.coordinates.lat
///     pub latitude: f64,
///     /// Longitude -- vehicle_state.location.coordinates.lon
///     pub longitude: f64,
///     /// Altitude -- vehicle_state.altitudes.geodetic
///     pub altitude: Option<f64>,
///     /// Distance to ground -- vehicle_state.altitudes.ato.value
///     pub elevation: Option<f64>,
///     /// Operator lat -- pilot_state.location.coordinates.lat
///     pub home_lat: Option<f64>,
///     /// Operator lon -- pilot_state.location.coordinates.lon
///     pub home_lon: Option<f64>,
///     /// Altitude from takeoff point -- (vehicle_state.altitudes.ato.value - )
///     pub home_height: Option<f64>,
///     /// Current speed -- vehicle_state.ground_speed
///     pub speed: f64,
///     /// True heading -- vehicle_state.orientation
///     pub heading: f64,
///     /// Vehicle state -- vehicle_state.state
///     pub state: Option<u8>,
///     /// Name of detecting point -- system.fusion_state.source_serials
///     pub station_name: Option<String>,
///
impl From<&FusedData> for DronePoint {
    fn from(value: &FusedData) -> Self {
        let station_name = value.system.fusion_state.source_serials[0].clone();

        Self {
            time: value.system.timestamp,
            journey: value.system.track_id.clone(),
            ident: value.vehicle_identification.serial.clone(),
            make: value.vehicle_identification.make.clone(),
            model: value.vehicle_identification.model.clone(),
            uav_type: value.vehicle_identification.uav_type,
            source: value.system.fusion_state.fusion_type,
            latitude: value.vehicle_state.location.coordinates.lat,
            longitude: value.vehicle_state.location.coordinates.lon,
            altitude: Some(
                value
                    .vehicle_state
                    .altitudes
                    .geodetic
                    .unwrap_or_default()
                    .into(),
            ),
            elevation: Some(value.vehicle_state.altitudes.ato.unwrap_or_default().into()),
            home_lat: Some(value.pilot_state.location.coordinates.lat),
            home_lon: Some(value.pilot_state.location.coordinates.lon),
            home_height: None,
            speed: value.vehicle_state.ground_speed.unwrap_or_default().into(),
            heading: value.vehicle_state.orientation.unwrap_or_default().into(),
            state: Some(value.vehicle_state.state.unwrap()),
            station_name: Some(station_name),
        }
    }
}
