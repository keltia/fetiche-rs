//! Module to load and process the Aeroscope data coming from ASD and generate
//! CSV data Cat21-like
//!

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{to_feet, to_knots, Bool, Cat21, Position, TodCalculated};

/// Our input structure from the csv file coming out of the aeroscope as CSV
///
#[derive(Debug, Deserialize)]
pub struct Aeroscope {
    // $1
    #[serde(rename = "aeroscope_id")]
    pub id: String,
    // $2 & $3
    pub aeroscope_position: Position,
    // $4
    pub altitude: f32,
    // $5
    pub azimuth: f32,
    // $6 & $7
    pub coordinate: Position,
    // $8
    pub distance: f32,
    // $9
    pub drone_id: String,
    // $10
    pub drone_type: String,
    // $11
    pub flight_id: String,
    // $12 & $13
    pub home_location: Position,
    // $14 & $15
    pub pilot_position: Position,
    // $16
    pub receive_date: String,
    // $17
    pub speed: f32,
}
