//! Module to handle Safesky data and map the input into our own Cat-21-like format.
//!

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::format::{to_feet, to_knots, Cat21};

/// Our input structure from the csv file coming from Safesky file
///
#[derive(Debug, Deserialize)]
pub struct Safesky {
    pub last_update: DateTime<Utc>,
    pub id: String,
    pub source: String,
    pub transponder_type: String,
    pub aircraft_type: String,
    pub latitude: f32,
    pub longitude: f32,
    pub altitude: u32,
    pub vertical_rate: i16,
    pub accuracy: u16,
    pub altitude_accuracy: i16,
    pub course: u16,
    pub ground_speed: u16,
    pub status: String,
    pub turn_rate: Option<String>,
    pub call_sign: String,
    pub ip: Option<IpAddr>,
}

impl From<&Safesky> for Cat21 {
    /// Minimal transformations for now.
    ///
    fn from(line: &Safesky) -> Self {
        let tod = line.last_update.timestamp();
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
            callsign: line.call_sign.to_owned(),
            groundspeed_kt: to_knots(line.ground_speed as f32),
            track_angle_deg: 0.0,
            rec_num: 1,
        }
    }
}
