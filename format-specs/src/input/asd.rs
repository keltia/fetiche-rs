//! Module to load and process the data coming from the ASD site and generate
//! CSV data Cat21-like
//!
//! Documentation is taken from `ASD_MAN_ManuelPositionnementAPI_v1.1.pdf`  as sent by ASD.
//!
//! JSON endpoint added later by ASD in Nov. 2022.

use chrono::NaiveDateTime;
use serde::Deserialize;

use crate::{to_feet, to_knots, Bool, Cat21, TodCalculated};

/// Our input structure from the json file coming out of the main ASD site
///
/// Data can be obtained either in CSV or JSON format-specs, we prefer the latter.
///
/// NOTE: Some fields are String and not the actual type (f32 for example) because there
/// are apparently stored as DECIMAL in their database and not as FLOAT.  There are then
/// exported as 6-digit floating strings.
///
#[derive(Debug, Deserialize)]
pub struct Asd {
    // Each record is part of a drone journey with a specific ID
    pub journey: u32,
    // Identifier for the drone
    pub ident: String,
    // Model of the drone
    pub model: Option<String>,
    // Source ([see src/site/asd.rs]) of the data
    pub source: String,
    // Point/record ID
    pub location: u32,
    // Date of event (in the non standard YYYY-MM-DD HH:MM:SS format-specs)
    pub timestamp: String,
    // $7 (actually f32)
    pub latitude: String,
    // $8 (actually f32)
    pub longitude: String,
    // Altitude, can be either null or negative (?)
    pub altitude: Option<i16>,
    // Distance to ground (estimated every 15s)
    pub elevation: Option<u32>,
    // Undocumented
    pub gps: Option<u32>,
    // Signal level (in dB)
    pub rssi: Option<i32>,
    // $13 (actually f32)
    pub home_lat: Option<String>,
    // $14 (actually f32)
    pub home_lon: Option<String>,
    // Altitude from takeoff point
    pub home_height: Option<f32>,
    // Current speed
    pub speed: f32,
    // True heading
    pub heading: f32,
    // Name of detecting point
    pub station_name: Option<String>,
    // Latitude (actually f32)
    pub station_lat: Option<String>,
    // Longitude (actually f32)
    pub station_lon: Option<String>,
}
