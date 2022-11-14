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
    pub model: Option<String>,
    // $4
    pub source: String,
    // $5
    pub location: u32,
    // $6
    pub timestamp: String,
    // $7 (actually f32)
    pub latitude: String,
    // $8 (actually f32)
    pub longitude: String,
    // $9
    pub altitude: Option<i16>,
    // $10
    pub elevation: Option<u32>,
    // $11
    pub gps: Option<u32>,
    // $12
    pub rssi: Option<i32>,
    // $13 (actually f32)
    pub home_lat: Option<String>,
    // $14 (actually f32)
    pub home_lon: Option<String>,
    // $15
    pub home_height: Option<f32>,
    // $16
    pub speed: f32,
    // $17
    pub heading: f32,
    // $18
    pub station_name: Option<String>,
    // $19 (actually f32)
    pub station_lat: Option<String>,
    // $20 (actually f32)
    pub station_lon: Option<String>,
}

impl From<&Asd> for Cat21 {
    /// Makes the loading and transformations
    ///
    /// The default values are arbitrary and taken from the original `aeroscope-CDG.sh` script
    /// by Marc Gravis.
    ///
    fn from(line: &Asd) -> Self {
        let tod = NaiveDateTime::parse_from_str(&line.timestamp, "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .timestamp();
        let alt_geo_ft = line.altitude.unwrap_or(0i16);
        let alt_geo_ft: f32 = alt_geo_ft.into();
        Cat21 {
            sac: 8,
            sic: 200,
            alt_geo_ft: to_feet(alt_geo_ft),
            pos_lat_deg: line.latitude.parse::<f32>().unwrap(),
            pos_long_deg: line.longitude.parse::<f32>().unwrap(),
            alt_baro_ft: to_feet(alt_geo_ft),
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
