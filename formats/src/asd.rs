//! Module to load and process the data coming from the ASD site and generate
//! CSV data Cat21-like
//!
//! Documentation is taken from `ASD_MAN_ManuelPositionnementAPI_v1.1.pdf`  as sent by ASD.
//!
//! JSON endpoint added later by ASD in Nov. 2022.

use chrono::{DateTime, NaiveDateTime, Utc};
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, PickFirst};

#[cfg(feature = "asterix")]
use crate::{convert_to, get_drone_id, to_feet, to_knots, Cat21, TodCalculated};

/// Our input structure from the json file coming out of the main ASD site
///
/// Data can be obtained in CSV or JSON formats, we prefer the former for size-related reasons.
///
/// NOTE: Some fields are String and not the actual type (f32 for example) because there
/// are apparently stored as DECIMAL in their database and not as FLOAT.  There are then
/// exported as 6-digit floating strings. `serde_as` is used to properly handle these.
///
/// `timestamp` format is NON-STANDARD so we had our own `time` field which gets ignored when
/// de-serialising and we fix it afterward.  We use `DateTime<Utc>` from `chrono` because plain
/// `i64` is not supported by InfluxDB as it is.
///
#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Asd {
    /// Hidden UNIX timestamp
    #[serde(skip_deserializing)]
    #[serde_as(as = "PickFirst<(_, DisplayFromStr)>")]
    pub time: DateTime<Utc>,
    /// Each record is part of a drone journey with a specific ID
    pub journey: u32,
    /// Identifier for the drone
    pub ident: String,
    /// Model of the drone
    pub model: Option<String>,
    /// Source ([see src/site/asd.rs]) of the data
    pub source: String,
    /// Point/record ID
    pub location: u32,
    /// Date of event (in the non standard YYYY-MM-DD HH:MM:SS formats)
    pub timestamp: String,
    /// $7 (actually f32)
    #[serde_as(as = "PickFirst<(_, DisplayFromStr)>")]
    pub latitude: f32,
    /// $8 (actually f32)
    #[serde_as(as = "PickFirst<(_, DisplayFromStr)>")]
    pub longitude: f32,
    /// Altitude, can be either null or negative (?)
    pub altitude: Option<i16>,
    /// Distance to ground (estimated every 15s)
    pub elevation: Option<i32>,
    /// Undocumented
    pub gps: Option<u32>,
    /// Signal level (in dB)
    pub rssi: Option<i32>,
    /// $13 (actually f32)
    #[serde_as(as = "PickFirst<(_, Option<DisplayFromStr>)>")]
    pub home_lat: Option<f32>,
    /// $14 (actually f32)
    #[serde_as(as = "PickFirst<(_, Option<DisplayFromStr>)>")]
    pub home_lon: Option<f32>,
    /// Altitude from takeoff point
    pub home_height: Option<f32>,
    /// Current speed
    pub speed: f32,
    /// True heading
    pub heading: f32,
    /// Name of detecting point
    pub station_name: Option<String>,
    /// Latitude (actually f32)
    #[serde_as(as = "PickFirst<(Option<_>, Option<DisplayFromStr>)>")]
    pub station_latitude: Option<f32>,
    /// Longitude (actually f32)
    #[serde_as(as = "PickFirst<(Option<_>, Option<DisplayFromStr>)>")]
    pub station_longitude: Option<f32>,
}

#[cfg(feature = "asterix")]
convert_to!(from_asd, Asd, Cat21);

impl Asd {
    /// Generate a proper timestamp from the non-standard string they emit.
    ///
    #[inline]
    pub fn fix_tm(&self) -> Result<Asd> {
        let tod = NaiveDateTime::parse_from_str(&self.timestamp, "%Y-%m-%d %H:%M:%S")?;
        let mut out = self.clone();
        out.time = tod.and_utc();
        Ok(out)
    }
}


#[cfg(feature = "asterix")]
impl From<&Asd> for Cat21 {
    /// Makes the loading and transformations
    ///
    /// The default values are arbitrary and taken from the original `aeroscope-CDG.sh` script
    /// by Marc Gravis.
    ///
    /// The following fields are **lost**:
    /// - journey
    /// - location
    /// - station_lat/lon
    /// - station_name
    /// - heading
    /// - home_lat/lon
    /// - home_height
    /// - model
    /// - gps
    /// - rssi
    ///
    #[tracing::instrument]
    fn from(line: &Asd) -> Self {
        let tod = line.time.timestamp();
        let alt_geo_ft = line.altitude.unwrap_or(0i16);
        let alt_geo_ft: f32 = alt_geo_ft.into();
        Cat21 {
            alt_geo_ft: to_feet(alt_geo_ft),
            pos_lat_deg: line.latitude,
            pos_long_deg: line.longitude,
            alt_baro_ft: to_feet(alt_geo_ft),
            tod: 128 * (tod % 86400),
            rec_time_posix: tod,
            emitter_category: 13,
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
            ..Cat21::default()
        }
    }
}
