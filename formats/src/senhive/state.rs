//! This is the module for data types for the `system_state` / `dl_system_state` queues
//!

use crate::senhive::Coordinates;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;

// ----- queue: `system_state`

/// Represents a service in the system along with its state and potential message.
///
/// # Fields
/// - `name` (String): The name of the service.
/// - `state` (String): The current state of the service (e.g., OK, ERROR, WARNING).
/// - `message` (Option<String>): An optional message providing additional details about the service's state.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub state: String,
    pub message: Option<String>,
}

/// Represents an altitude record containing optional measurements of altitude values above
/// ground level (`agl`), above mean sea level (`amsl`), and geodetic altitude.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Altitude {
    pub agl: Option<f64>,
    pub amsl: Option<f64>,
    pub geodetic: Option<f64>,
}

/// Represents the types of sensors supported by the system.
///
/// Each sensor type corresponds to specific hardware or software sensors
/// used in the `system_state` or `dl_system_state` queues. The types of
/// sensors supported are:
///
/// - `SENID`: A basic sensor for identification purposes.
/// - `SENID_PLUS`: An upgraded version of the SENID sensor.
/// - `SENIRIS`: A sensor for advanced detection capabilities.
/// - `RADAR`: A radar-based sensor.
/// - `RF`: A radio frequency sensor.
/// - `AUDIO`: An audio-based sensor.
/// - `CAMERA`: A sensor used for visual detection.
/// - `UNKNOWN`: Used when the sensor type cannot be determined.
///
/// Serialized in SCREAMING_SNAKE_CASE format.
///
#[derive(Debug, Deserialize, strum::Display, EnumString, strum::VariantNames, Serialize)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum SensorType {
    Senid,
    SenidPlus,
    Seniris,
    Radar,
    Rf,
    Audio,
    Camera,
    Unknown,
}

/// Represents the different states that a system or component can be in.
///
/// Serialized in `UPPERCASE` format.
///
/// - `UNKNOWN`: The state is not known or cannot be determined.
/// - `ERROR`: Indicates a critical problem in the system or component.
/// - `WARNING`: A cautionary state showing that attention might be needed.
/// - `OK`: Everything is functioning as expected and within normal parameters.
///
#[derive(Debug, Deserialize, strum::Display, EnumString, strum::VariantNames, Serialize)]
#[strum(serialize_all = "UPPERCASE")]
pub enum State {
    Unknown,
    Error,
    Warning,
    Ok,
}

/// Represents a sensor and its associated properties.
///
/// # Fields
/// - `serial` (String): A unique identifier for the sensor.
/// - `name` (String): The name of the sensor.
/// - `type` (String): The type of sensor (e.g., SENID, RADAR).
/// - `coordinates` (Option<Coordinates>): The geographic coordinates (latitude, longitude) where the sensor is located.
/// - `altitude` (Altitude): Stores information about the sensor's altitude above the ground, mean sea level, or geodetic altitude.
/// - `estimated_coverage` (Option<String>): Represents the area covered by the sensor, specified as a WKT polygon.
/// - `state` (String): The current state of the sensor (e.g., OK, WARNING, ERROR).
/// - `last_online` (Option<DateTime<Utc>>): The last known time the sensor was online.
/// - `meta_data` (Option<String>): Additional meta-information related to the sensor, serialized as a string.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Sensor {
    pub serial: String,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub coordinates: Option<Coordinates>,
    pub altitude: Altitude,
    /// Defined as a WKT POLYGON
    #[serde(rename = "estimatedCoverage")]
    pub estimated_coverage: Option<String>,
    pub state: String,
    #[serde(rename = "lastOnline")]
    pub last_online: Option<DateTime<Utc>>,
    #[serde(rename = "metaData")]
    pub meta_data: Option<String>,
}

/// Message sent through the `system_state` topic, every minute.
///
/// Represents the message providing the overall system state information including sensors and services.
///
/// # Fields
///
/// - `version` (Option<String>): The version of the message or protocol being used.
/// - `name` (String): The name of the system or entity sending the message.
/// - `timestamp` (DateTime<Utc>): The time when the message was generated.
/// - `sensors` (Vec<Sensor>): A list of sensors attached to the system, with their details.
/// - `services` (Vec<Service>): A list of services running in the system, along with their states.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct StateMsg {
    pub version: Option<String>,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub sensors: Vec<Sensor>,
    pub services: Vec<Service>,
}
