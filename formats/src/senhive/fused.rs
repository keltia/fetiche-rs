//! This is the module for the data types for the `fused_data` / `dl_fused_data` queues.
//!

// ----- queue: `fused_data`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::DronePoint;
use crate::senhive::Coordinates;

// ----- Original raw data format

/// Represents a location with geographic coordinates, uncertainty, and an optional likelihood.
///
/// # Fields
/// - `coordinates` (Coordinates): The geographic coordinates (latitude and longitude) of the location.
/// - `uncertainty` (Option<f64>): The uncertainty in meters associated with the location. Optional.
/// - `likelihood` (Option<String>): A WKT Polygon string representing the likelihood of the location. Optional.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub coordinates: Coordinates,
    pub uncertainty: Option<f64>,
    /// This is a string with a 7-point WKT Polygon
    pub likelihood: Option<String>,
}

/// Enumeration representing the type of location associated with a measurement.
///
/// # Variants
/// - `TakeOff` (0): Represents the take-off location.
/// - `Home` (1): Represents the UAV home location.
/// - `Live` (2): Represents a live measurement update.
/// - `Unknown` (15): Represents an unknown location type. This is the default variant.
///
#[derive(Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum LocationType {
    TakeOff = 0,
    Home = 1,
    Live = 2,
    #[default]
    Unknown = 15,
}

/// Represents the state of the pilot in the `fused_data` / `dl_fused_data` queues.
///
/// # Fields
///
/// - `location` (Location): The current geographical location of the pilot, including coordinates, uncertainty, and likelihood.
/// - `location_type` (u8): The type of location represented, such as take-off, home, live, or unknown.
///   Serialized under the field name `locationType`.
///
/// This struct provides a way to represent the pilot’s state, such as their location and an
/// associated location type. The `location` field includes additional related metadata for clarity.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct PilotState {
    pub location: Location,
    #[serde(rename = "locationType")]
    pub location_type: u8,
}

/// Represents identification details of a pilot in the system.
///
/// This struct provides metadata about a pilot, including the unique identifier,
/// name, and optional geographical location. It can be used to associate
/// operational data with a specific pilot's information.
///
/// # Fields
///
/// - `id` (u64): Unique identifier for the pilot.
/// - `name` (String): The full name of the pilot.
/// - `location` (Option<Location>): Optional geographical location of the pilot,
///    represented as latitude and longitude coordinates.
///
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct PilotIdentification {
    pub id: u64,
    pub name: String,
    pub location: Option<Location>,
}

/// Represents a numerical value with an associated uncertainty.
///
/// This struct is used to encapsulate a value and its potential uncertainty, allowing
/// more precise representation of measurements or calculations that involve
/// a margin of error or variability in recorded data.
///
/// # Fields
/// - `value` (f64): The primary numerical value.
/// - `uncertainty` (Option<f64>): An optional uncertainty that quantifies how far the actual value might deviate
///   from the recorded `value`. Typically measured as a range (±) around the value.
///
/// # Traits
/// - `Default`: When the default value is specified, `FusedValue` initializes with:
///   - `value`: `0.0`
///   - `uncertainty`: `None`
/// - `From<FusedValue> for f64`: Provides an easy way to extract the `value` as a plain `f64`.
///
/// This struct is commonly used in scenarios where measurements or predictions
/// include an inherent level of uncertainty, such as in sensor data or scientific computations.
///
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

/// Represents altitude measurements with associated uncertainties.
///
/// This struct is used to encapsulate altitude data from various reference points,
/// enabling precise representation of altitude measurements in different contexts.
///
/// # Fields
///
/// - `ato` (Option<FusedValue>): Altitude above the take-off location in meters. Optional.
/// - `agl` (Option<FusedValue>): Altitude above the ground level in meters. Optional.
/// - `amsl` (Option<FusedValue>): Altitude above mean sea level in meters. Optional.
/// - `geodetic` (Option<FusedValue>): The real geodetic altitude in meters. Optional.
///
/// Each field represents a specific altitude measurement, and the associated `FusedValue`
/// includes an optional uncertainty to represent the measurement's accuracy.
///
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

/// Represents the state of the vehicle, including its location, altitude information,
/// speed, orientation, and overall status.
///
/// This struct is used to provide detailed telemetry data about the vehicle in the system,
/// including positional and movement-related metrics.
///
/// # Fields
///
/// - `location` (Location): The current geographical location of the vehicle, including coordinates and related metadata.
/// - `altitudes` (Altitudes): Altitude measurements of the vehicle relative to different reference points (e.g., ground level, sea level).
/// - `ground_speed` (Option<FusedValue>): The current horizontal speed of the vehicle over the ground, optionally including uncertainty.
/// - `vertical_speed` (Option<FusedValue>): The current vertical speed of the vehicle (rate of ascent or descent), optionally including uncertainty.
/// - `orientation` (Option<FusedValue>): The orientation of the vehicle, typically expressed as a heading or directional angle, optionally with uncertainty.
/// - `state` (Option<u8>): An optional state indicator for the vehicle. This could represent specific states (e.g., active, inactive, error) if defined in the system.
///
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

/// Contains the attributes required for identifying a vehicle in the system.
///
/// This struct provides metadata about a vehicle, such as its serial number,
/// MAC address, and hardware details, along with its type.
///
/// # Fields
///
/// - `serial` (Option<String>): The serial number identifying the vehicle. Optional.
/// - `mac` (Option<String>): The MAC address associated with the vehicle. Optional.
/// - `make` (Option<String>): The manufacturer or brand of the vehicle. Optional.
/// - `model` (Option<String>): The specific model of the vehicle. Optional.
/// - `uav_type` (u8): Represents the UAV type of the vehicle. Required.
///
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

/// Represents the state of a fusion process, including the type of fusion
/// and the serials of sources involved in the fusion process.
///
/// # Fields
///
/// - `fusion_type` (u8): Represents the type of fusion being conducted.
/// - `source_serials` (Vec<String>): A list of source serial numbers participating in the fusion process.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct FusionState {
    #[serde(rename = "fusionType")]
    pub fusion_type: u8,
    #[serde(rename = "sourceSerials")]
    pub source_serials: Vec<String>,
}

/// Represents the overall system state, including fusion data, vehicle details,
/// and pilot-related information.
///
/// This struct is designed to contain data generated by a series of fused records.
/// It provides comprehensive telemetry, state, and identification information for
/// operational drones or vehicles along with the pilot’s location and status.
///
/// # Fields
///
/// - `version` (String): The version of the fused data format.
/// - `system` (System): Details about the system including tracking ID, timestamps, and fusion state.
/// - `vehicle_identification` (VehicleIdentification): Metadata regarding the vehicle (serial, MAC, make, etc.).
/// - `vehicle_state` (VehicleState): Telemetry data describing the current state of the vehicle, such as location, altitudes, and speed.
/// - `pilot_identification` (Option<PilotIdentification>): Optional information about the pilot.
/// - `pilot_state` (PilotState): The pilot's current state, including location and other metadata.
///
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

/// Represents a log entry for the timestamped processes in the system.
///
/// This struct provides information about a specific process that occurred
/// in the system, including the name of the process, the time it occurred,
/// and an optional message providing additional details.
///
/// # Fields
/// - `process_name` (String): The name of the process.
/// - `timestamp` (DateTime<Utc>): The time at which the process occurred.
/// - `msg` (Option<String>): An optional message providing additional context or details
///   about the process.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct TSLog {
    pub process_name: String,
    pub timestamp: DateTime<Utc>,
    pub msg: Option<String>,
}

/// Represents a fused data structure containing comprehensive telemetry,
/// state, and identification information for operational drones or vehicles,
/// fused from multiple data inputs.
///
/// This structure is designed to encapsulate data related to the state of a
/// vehicle (drones, UAVs, etc.), the pilot's information, and the system's
/// overall state.
///
/// # Fields
///
/// - `version` (String): The version of the fused data format.
/// - `system` (System): Details about the tracking system, including the ID,
///   timestamps, and the fusion state.
/// - `vehicle_identification` (VehicleIdentification): Contains metadata
///   about the vehicle, such as its serial, MAC address, make, model, and UAV type.
/// - `vehicle_state` (VehicleState): Telemetry data describing the vehicle’s
///   current status, which includes location, altitudes, speed, orientation, and state.
/// - `pilot_identification` (Option<PilotIdentification>): Optional metadata
///   about the pilot, including their unique identifier.
/// - `pilot_state` (PilotState): Current state and location information about
///   the pilot.
///
/// This struct provides an aggregated view of multiple sources of information
/// and serves as a unified format for telemetry and operation details.
///
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
/// ```rust,no_run
/// use chrono::{DateTime, Utc};
///
/// struct DroneData {
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
/// }
/// ```
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
