//! Module for Thales Senhive data feed.
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Service {
    pub name: String,
    pub state: String,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Altitude {
    pub agl: Option<f64>,
    pub amsl: Option<f64>,
    pub geodetic: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Coordinates {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Sensor {
    pub serial: String,
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub coordinates: Coordinates,
    pub altitude: Altitude,
    #[serde(rename = "estimatedCoverage")]
    pub estimated_coverage: Option<f64>,
    pub state: String,
    #[serde(rename = "lastOnline")]
    pub last_online: DateTime<Utc>,
    #[serde(rename = "metaData")]
    pub meta_data: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StateMsg {
    pub version: String,
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub sensors: Vec<Sensor>,
    pub services: Vec<Service>,
}
