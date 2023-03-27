//! Module to load and process the Aeroscope data coming from ASD and generate
//! CSV data Cat21-like
//!

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{to_feet, to_knots, Bool, Cat129, Cat21, Position, TodCalculated, DEF_SAC, DEF_SIC};

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
            differential_correction: Bool::N,
            ground_bit: Bool::N,
            simulated_target: Bool::N,
            test_target: Bool::N,
            from_ft: Bool::N,
            selected_alt_capability: Bool::N,
            spi: Bool::N,
            link_technology_cddi: Bool::N,
            link_technology_mds: Bool::N,
            link_technology_uat: Bool::N,
            link_technology_vdl: Bool::N,
            link_technology_other: Bool::N,
            descriptor_atp: 1,
            alt_reporting_capability_ft: 0,
            target_addr: 623615,
            cat: 21,
            line_id: 1,
            ds_id: 18,
            report_type: 3,
            tod_calculated: TodCalculated::N,
            // We do truncate the drone_id for privacy reasons
            callsign: lid,
            groundspeed_kt: to_knots(line.speed),
            track_angle_deg: line.azimuth,
            rec_num: 1,
        }
    }
}

impl From<&Aeroscope> for Cat129 {
    /// Load and transform into Cat129
    ///
    fn from(line: &Aeroscope) -> Self {
        let tod = line.receive_date.parse::<DateTime<Utc>>().unwrap();
        let tod = tod.timestamp();
        let lid = if line.drone_id != "null" {
            line.drone_id[2..10].to_owned()
        } else {
            "null".to_owned()
        };
        Cat129 {
            // XXX This is obviously wrong
            sac: DEF_SAC,
            sic: DEF_SIC,
            dac: DEF_SAC,
            dic: DEF_SIC,
            uas_manufacturer_id: "DJI".to_string(),
            uas_model_id: line.drone_type.to_owned(),
            uas_serial: lid,
            uas_reg_country: "fr".to_string(),
            tod,
            position: line.coordinate,
            alt_sea_lvl: line.altitude,
            alt_gnd_lvl: line.altitude,
            gnss_acc: 1.0,
            ground_speed: to_knots(line.speed),
            vert_speed: 1.0,
        }
    }
}
