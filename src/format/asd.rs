//! Module to load and process the data coming from the ASD site and generate
//! CSV data Cat21-like
//!

use chrono::NaiveDateTime;
use serde::Deserialize;

use crate::format::{to_feet, to_knots, Cat21};

/// Our input structure from the csv file coming out of the aeroscope
///
#[derive(Debug, Deserialize)]
pub struct Asd {
    // $1
    pub journey: u32,
    // $2
    pub ident: String,
    // $3
    pub model: String,
    // $4
    pub source: String,
    // $5
    pub location: u32,
    // $6
    pub timestamp: NaiveDateTime,
    // $7
    pub latitude: f32,
    // $8
    pub longitude: f32,
    // $9
    pub altitude: u16,
    // $10
    pub elevation: Option<String>,
    // $11
    pub gps: u32,
    // $12
    pub rssi: Option<String>,
    // $13
    pub home_lat: Option<f32>,
    // $14
    pub home_lon: Option<f32>,
    // $15
    pub home_height: Option<f32>,
    // $16
    pub speed: f32,
    // $17
    pub heading: f32,
    // $18
    pub station_name: String,
    // $19
    pub station_lat: f32,
    // $20
    pub station_lon: f32,
}

impl From<Asd> for Cat21 {
    /// Makes the loading and transformations
    ///
    /// The default values are arbitrary and taken from the original `aeroscope-CDG.sh` script
    /// by Marc Gravis.
    ///
    fn from(line: Asd) -> Self {
        let tod = line.timestamp.timestamp();
        Cat21 {
            sac: 8,
            sic: 200,
            alt_geo_ft: to_feet(line.altitude as f32),
            pos_lat_deg: line.latitude,
            pos_long_deg: line.longitude,
            alt_baro_ft: to_feet(line.altitude as f32),
            tod: 128 * (tod % 86400),
            rec_time_posix: tod,
            rec_time_ms: 0,
            emitter_category: 13,
            differential_correction: "N".to_string(),
            ground_bit: "N".to_string(),
            simulated_target: "N".to_string(),
            test_target: "N".to_string(),
            from_ft: "N".to_string(),
            selected_alt_capability: "N".to_string(),
            spi: "N".to_string(),
            link_technology_cddi: "N".to_string(),
            link_technology_mds: "N".to_string(),
            link_technology_uat: "N".to_string(),
            link_technology_vdl: "N".to_string(),
            link_technology_other: "N".to_string(),
            descriptor_atp: 1,
            alt_reporting_capability_ft: 0,
            target_addr: 623615,
            cat: 21,
            line_id: 1,
            ds_id: 18,
            report_type: 3,
            tod_calculated: "N".to_string(),
            // We do truncate the drone_id for privacy reasons
            callsign: line.ident[2..10].to_owned(),
            groundspeed_kt: to_knots(line.speed),
            track_angle_deg: line.heading,
            rec_num: 1,
        }
    }
}
