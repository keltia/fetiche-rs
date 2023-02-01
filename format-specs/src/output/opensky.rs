//! Module to load and process the data coming from the OpenskyD site and generate
//! CSV data Cat21-like
//!
//! XXX they send out an array of arrays, each representing a specific state vector.
//!     it sucks.
//!
//! XXX Due to this, I'm not sure converting these state vectors into our Cat21 makes any sense.
//!
//! Documentation is taken from [The Opensky site](https://opensky-network.github.io/opensky-api/rest.html)
//!

use chrono::{DateTime, Utc};

use crate::input::opensky::StateVector;
use crate::{to_feet, to_knots, Bool, Cat21, TodCalculated};

impl From<&StateVector> for Cat21 {
    fn from(line: &StateVector) -> Self {
        let tp = format!("{}", line.time_position.unwrap_or(0));
        let tod = tp.parse::<DateTime<Utc>>().unwrap();
        let tod = tod.timestamp();
        let callsign = line.callsign.clone().unwrap_or("".to_string());

        Cat21 {
            sac: 8,
            sic: 200,
            alt_geo_ft: to_feet(line.geo_altitude.unwrap_or(0.0)),
            pos_lat_deg: line.latitude.unwrap_or(0.0),
            pos_long_deg: line.longitude.unwrap_or(0.0),
            alt_baro_ft: to_feet(line.baro_altitude.unwrap_or(0) as f32),
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
            callsign,
            groundspeed_kt: to_knots(line.velocity.unwrap_or(0) as f32),
            track_angle_deg: line.true_track.unwrap_or(0.0),
            rec_num: 1,
        }
    }
}
