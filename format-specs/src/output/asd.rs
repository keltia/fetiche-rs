//! Module to load and process the data coming from the ASD site and generate
//! CSV data Cat21-like
//!
//! Documentation is taken from `ASD_MAN_ManuelPositionnementAPI_v1.1.pdf`  as sent by ASD.
//!
//! JSON endpoint added later by ASD in Nov. 2022.

use chrono::NaiveDateTime;

use crate::input::asd::Asd;
use crate::{to_feet, to_knots, Bool, Cat21, TodCalculated};

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
            callsign: get_drone_id(&line.ident),
            groundspeed_kt: to_knots(line.speed),
            track_angle_deg: line.heading,
            rec_num: 1,
        }
    }
}
