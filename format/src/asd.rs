//! Module to load and process the data coming from the ASD site and generate
//! CSV data Cat21-like
//!
//! Documentation is taken from `ASD_MAN_ManuelPositionnementAPI_v1.1.pdf`  as sent by ASD.
//!
//! JSON endpoint added later by ASD in Nov. 2022.

use chrono::NaiveDateTime;
use serde::Deserialize;

use crate::{to_feet, to_knots, Cat21};

/// Our input structure from the json file coming out of the main ASD site
///
/// Data can be obtained either in CSV or JSON format, we prefer the latter.
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
    // Date of event (in the non standard YYYY-MM-DD HH:MM:SS format)
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

/// For privacy reasons, we truncate the drone ID value to something not unique
///
#[cfg(feature = "privacy")]
fn get_drone_id(id: &str) -> String {
    id[2..10].to_owned()
}

#[cfg(not(feature = "privacy"))]
fn get_drone_id(id: &str) -> String {
    id.to_owned()
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
            callsign: get_drone_id(&line.ident),
            groundspeed_kt: to_knots(line.speed),
            track_angle_deg: line.heading,
            rec_num: 1,
        }
    }
}
