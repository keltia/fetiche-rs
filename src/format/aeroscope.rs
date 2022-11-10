//! Module to load and process the Aeroscope data coming from ASD and generate
//! CSV data Cat21-like
//!

use chrono::{DateTime, Utc};
use log::debug;
use serde::Deserialize;

use crate::format::{to_feet, to_knots, Cat21, Position};

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

impl From<&Aeroscope> for Cat21 {
    /// Makes the loading and transformations
    ///
    /// The default values are arbitrary and taken from the original `aeroscope.sh` script
    /// by Marc Gravis.
    ///
    fn from(line: &Aeroscope) -> Self {
        debug!("Converting from {:?}", line);
        let tod = line.receive_date.parse::<DateTime<Utc>>().unwrap();
        let tod = tod.timestamp();
        let lid = if line.drone_id != "null" {
            line.drone_id[2..10].to_owned()
        } else {
            "null".to_owned()
        };
        Cat21 {
            sac: 8,
            sic: 200,
            alt_geo_ft: to_feet(line.altitude),
            pos_lat_deg: line.coordinate.latitude,
            pos_long_deg: line.coordinate.longitude,
            alt_baro_ft: to_feet(line.altitude),
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
            callsign: lid,
            groundspeed_kt: to_knots(line.speed),
            track_angle_deg: line.azimuth,
            rec_num: 1,
        }
    }
}
