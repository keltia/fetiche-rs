//! Module to handle Safesky data and map the input into our own Cat-21-like formats.
//!
//! Phases: (TBC)
//! - use the API key configured in the configuration file to fetch data
//!
//! The file given to us as example is apparently from the `/v1/beacons`  endpoint as it contains
//! only ADS-BI (see `Safesky.transponder_type`) data.
//!

use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{to_feet, to_knots, Bool, Cat21, TodCalculated};

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

impl From<&Safesky> for Cat21 {
    /// Generate a `Cat21` struct from Safesky..
    ///
    /// TODO: transformations to be confirmed
    ///
    #[tracing::instrument]
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
            callsign: line.call_sign.to_owned(),
            groundspeed_kt: to_knots(line.ground_speed as f32),
            track_angle_deg: 0.0,
            rec_num: 1,
        }
    }
}
