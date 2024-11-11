//! This is the module for data types for the `system_state` / `dl_system_state` queues
//!

use crate::senhive::Coordinates;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;

// ----- queue: `system_state`

#[derive(Debug, Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub state: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Altitude {
    pub agl: Option<f64>,
    pub amsl: Option<f64>,
    pub geodetic: Option<f64>,
}

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

#[derive(Debug, Deserialize, strum::Display, EnumString, strum::VariantNames, Serialize)]
#[strum(serialize_all = "UPPERCASE")]
pub enum State {
    Unknown,
    Error,
    Warning,
    Ok,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sensor {
    pub serial: String,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: SensorType,
    pub coordinates: Option<Coordinates>,
    pub altitude: Altitude,
    /// Defined as a WKT POLYGON
    #[serde(rename = "estimatedCoverage")]
    pub estimated_coverage: Option<String>,
    pub state: State,
    #[serde(rename = "lastOnline")]
    pub last_online: Option<DateTime<Utc>>,
    #[serde(rename = "metaData")]
    pub meta_data: Option<String>,
}

/// Message sent through the `system_state` topic, every minute.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct StateMsg {
    pub version: Option<String>,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub sensors: Vec<Sensor>,
    pub services: Vec<Service>,
}

