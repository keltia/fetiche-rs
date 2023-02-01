//! Module to handle Safesky data and map the input into our own Cat-21-like format-specs.
//!
//! Phases: (TBC)
//! - use the API key configured in the configuration file to fetch data
//!
//! The file given to us as example is apparently from the `/v1/beacons`  endpoint as it contains
//! only ADS-BI (see `Safesky.transponder_type`) data.
//!
//! This implement the `Fetchable` trait described in `site/lib`.
//!

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Our input structure from the csv file coming from Safesky file
///
#[derive(Debug, Deserialize)]
pub struct Safesky {
    /// UTC Timestamp
    pub last_update: DateTime<Utc>,
    /// ID of the station ?
    pub id: String,
    /// Apparently always "safesky"
    pub source: String,
    /// For beacons, it should be "ADS-BI"
    pub transponder_type: String,
    pub aircraft_type: String,
    pub latitude: f32,
    pub longitude: f32,
    pub altitude: u32,
    pub vertical_rate: i16,
    pub accuracy: u16,
    pub altitude_accuracy: i16,
    /// Heading
    pub course: u16,
    pub ground_speed: u16,
    /// "AIRBORNE", etc.
    pub status: String,
    pub turn_rate: Option<String>,
    pub call_sign: String,
    pub ip: Option<IpAddr>,
}
