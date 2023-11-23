//! Module to load and process the data coming from the ASD site and generate
//! CSV data Cat21-like
//!
//! Documentation is taken from `ASD_MAN_ManuelPositionnementAPI_v1.1.pdf`  as sent by ASD.
//!
//! JSON endpoint added later by ASD in Nov. 2022.

use chrono::NaiveDateTime;
use eyre::Result;
use parquet_derive::ParquetRecordWriter;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tracing::debug;

use crate::drone::DronePoint;
use crate::{convert_to, to_feet, to_knots, Bool, Cat21, TodCalculated};

/// Our input structure from the json file coming out of the main ASD site
///
/// Data can be obtained either in CSV or JSON formats, we prefer the latter.
///
/// NOTE: Some fields are String and not the actual type (f32 for example) because there
/// are apparently stored as DECIMAL in their database and not as FLOAT.  There are then
/// exported as 6-digit floating strings.
///
/// `timestamp` format is NON-STANDARD so we had out own `tm` field which gets ignored when
/// de-serialising and we fix it afterward
///
#[serde_as]
#[derive(Debug, Deserialize, ParquetRecordWriter, Serialize)]
pub struct Asd {
    /// Hidden UNIX timestamp
    #[serde(skip_deserializing)]
    pub tm: i64,
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
    #[serde_as(as = "DisplayFromStr")]
    pub latitude: f32,
    /// $8 (actually f32)
    #[serde_as(as = "DisplayFromStr")]
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
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub home_lat: Option<f32>,
    /// $14 (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
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
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub station_latitude: Option<f32>,
    /// Longitude (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub station_longitude: Option<f32>,
}

convert_to!(from_asd, Asd, Cat21);
convert_to!(from_asd, Asd, DronePoint);

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
    /// The following fields are lost:
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
        let tod = NaiveDateTime::parse_from_str(&line.timestamp, "%Y-%m-%d %H:%M:%S")
            .unwrap()
            .timestamp();
        let alt_geo_ft = line.altitude.unwrap_or(0i16);
        let alt_geo_ft: f32 = alt_geo_ft.into();
        Cat21 {
            sac: 8,
            sic: 200,
            alt_geo_ft: to_feet(alt_geo_ft),
            pos_lat_deg: line.latitude,
            pos_long_deg: line.longitude,
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

#[inline]
fn safe_coord(s: Option<f32>) -> Option<f32> {
    Some(s.unwrap_or(0.0))
}

impl From<&Asd> for DronePoint {
    #[tracing::instrument]
    fn from(value: &Asd) -> Self {
        // Transform the string into proper timestamp
        //
        let tod = NaiveDateTime::parse_from_str(&value.timestamp, "%Y-%m-%d %H:%M:%S").unwrap();
        let tod = tod.and_utc().timestamp();

        DronePoint {
            time: tod,
            journey: value.journey,
            drone_id: get_drone_id(&value.ident),
            model: value.model.clone(),
            source: value.source.clone(),
            location: value.location,
            latitude: value.latitude,
            longitude: value.longitude,
            altitude: value.altitude,
            elevation: value.elevation,
            home_lat: safe_coord(value.home_lat),
            home_lon: safe_coord(value.home_lon),
            home_height: value.home_height,
            speed: value.speed,
            heading: value.heading,
            station_name: value.station_name.clone(),
            station_lat: safe_coord(value.station_latitude),
            station_lon: safe_coord(value.station_longitude),
        }
    }
}
