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

/// Represents a record obtained from the ASD site.
///
/// Data can be obtained in CSV or JSON formats, we prefer the former for size-related reasons.
///
/// This struct defines the data schema for the JSON input provided by ASD,
/// which includes various fields such as drone information, location details,
/// altitude, speed, and signal strength, among others.
///
/// Some fields are String and not the actual type (f32 for example) because there
/// are apparently stored as DECIMAL in their database and not as FLOAT.  There are then
/// exported as 6-digit floating strings. `serde_as` is used to properly handle these.
///
/// Fields with the `Option` type indicate that the data is either optional
/// or may occasionally contain null values.
///
/// The `time` field is computed afterwards from the `timestamp` field,
/// which is provided in a non-standard string format. The corrected `time`
/// is represented using `DateTime<Utc>` from the `chrono` crate.
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
    /// Fixes the timestamp by converting the non-standard string representation of the timestamp
    /// into a proper `DateTime<Utc>` instance.
    ///
    /// # Returns
    ///
    /// * `Ok(Asd)` - On successful conversion, it returns a new `Asd` object with the corrected timestamp.
    /// * `Err(eyre::Report)` - If the timestamp is invalid or cannot be parsed, it returns an error.
    ///
    /// # Examples
    ///
    /// Valid timestamp example:
    /// ```
    /// use chrono::{TimeZone, Utc};
    /// use fetiche_formats::Asd;
    ///
    /// let asd = Asd {
    ///     time: Utc::now(),
    ///     journey: 42,
    ///     ident: "Drone123".to_string(),
    ///     model: Some("ModelX".to_string()),
    ///     source: "ASD Source".to_string(),
    ///     location: 1,
    ///     timestamp: "2023-10-22 15:30:45".to_string(),
    ///     latitude: 48.8566,
    ///     longitude: 2.3522,
    ///     altitude: Some(120),
    ///     elevation: Some(60),
    ///     gps: Some(1),
    ///     rssi: Some(-85),
    ///     home_lat: Some(48.8566),
    ///     home_lon: Some(2.3522),
    ///     home_height: Some(100.0),
    ///     speed: 15.0,
    ///     heading: 90.0,
    ///     station_name: Some("Station1".to_string()),
    ///     station_latitude: Some(48.8570),
    ///     station_longitude: Some(2.3530),
    /// };
    ///
    /// let result = asd.fix_tm();
    /// assert!(result.is_ok());
    ///
    /// let fixed_asd = result.unwrap();
    /// assert_eq!(
    ///     fixed_asd.time,
    ///     Utc.with_ymd_and_hms(2023, 10, 22, 15, 30, 45).unwrap()
    /// );
    /// ```
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
    /// Converts an `Asd` instance to a `Cat21` instance.
    ///
    /// The default values are arbitrary and taken from the original `aeroscope-CDG.sh` script
    /// by Marc Gravis.
    ///
    /// This function performs transformations and field mapping between the `Asd`
    /// and `Cat21` structures. Some fields are retained, while others are either
    /// discarded or transformed as follows:
    ///
    /// #### Mapped Fields:
    /// - `alt_geo_ft`: Converted from `Asd`'s `altitude` field and transformed into feet.
    /// - `pos_lat_deg`: Mapped directly from `Asd`'s `latitude` field.
    /// - `pos_long_deg`: Mapped directly from `Asd`'s `longitude` field.
    /// - `tod`: Computed using the `time` field, transformed for CAT21 compatibility.
    /// - `rec_time_posix`: Direct timestamp from the `time` field.
    /// - `callsign`: Derived and truncated from the `ident` field for privacy.
    /// - `groundspeed_kt`: Converted from `speed` in the `Asd` instance.
    /// - `track_angle_deg`: Mapped from `heading`.
    ///
    /// #### Unused Fields:
    /// The following fields from `Asd` are **not transferred** to the `Cat21` structure:
    /// - `journey`: Not applicable to `Cat21`.
    /// - `location`: Dropped during transformation.
    /// - `station_lat`, `station_lon`, `station_name`: Specific data for `Asd` stations.
    /// - `heading`: Not required for this transformation.
    /// - `home_lat`, `home_lon`, `home_height`: Selective drone-specific data omitted here.
    /// - `model`: Only applicable to `Asd`.
    /// - `gps`: Low-impact field dropped.
    /// - `rssi`: Not applicable for CAT21.
    ///
    /// #### Truncated Fields:
    /// - `callsign` is truncated for privacy considerations before the transformation.
    ///
    /// The default values for some `Cat21` fields are arbitrary and follow the standard
    /// defined by the original `aeroscope-CDG.sh` script by Marc Gravis.
    ///
    /// #### Example Usage:
    /// ```
    /// use chrono::Utc;
    /// use fetiche_formats::{Asd, Cat21};
    ///
    /// let asd = Asd {
    ///     time: Utc::now(),
    ///     journey: 42,
    ///     ident: "Drone123".to_string(),
    ///     model: Some("ModelX".to_string()),
    ///     source: "ASD Source".to_string(),
    ///     location: 1,
    ///     timestamp: "2023-10-22 15:30:45".to_string(),
    ///     latitude: 48.8566,
    ///     longitude: 2.3522,
    ///     altitude: Some(120),
    ///     elevation: Some(60),
    ///     gps: Some(1),
    ///     rssi: Some(-85),
    ///     home_lat: Some(48.8566),
    ///     home_lon: Some(2.3522),
    ///     home_height: Some(100.0),
    ///     speed: 15.0,
    ///     heading: 90.0,
    ///     station_name: Some("Station1".to_string()),
    ///     station_latitude: Some(48.8570),
    ///     station_longitude: Some(2.3530),
    /// };
    ///
    /// let cat21: Cat21 = Cat21::from(&asd);
    /// ```
    ///
    /// **Note:** This is part of the `asterix` feature module and will only be available
    /// if the feature is enabled. The conversion also uses helper functions like `to_feet`
    /// and `to_knots` for field transformations.
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_fix_tm_valid_timestamp() {
        let asd = Asd {
            time: Utc::now(),
            journey: 42,
            ident: "Drone123".to_string(),
            model: Some("ModelX".to_string()),
            source: "ASD Source".to_string(),
            location: 1,
            timestamp: "2023-10-22 15:30:45".to_string(),
            latitude: 48.8566,
            longitude: 2.3522,
            altitude: Some(120),
            elevation: Some(60),
            gps: Some(1),
            rssi: Some(-85),
            home_lat: Some(48.8566),
            home_lon: Some(2.3522),
            home_height: Some(100.0),
            speed: 15.0,
            heading: 90.0,
            station_name: Some("Station1".to_string()),
            station_latitude: Some(48.8570),
            station_longitude: Some(2.3530),
        };

        let result = asd.fix_tm();
        assert!(result.is_ok());

        let fixed_asd = result.unwrap();
        assert_eq!(
            fixed_asd.time,
            Utc.with_ymd_and_hms(2023, 10, 22, 15, 30, 45).unwrap()
        );
    }

    #[test]
    fn test_fix_tm_invalid_timestamp() {
        let asd = Asd {
            time: Utc::now(),
            journey: 42,
            ident: "Drone123".to_string(),
            model: Some("ModelX".to_string()),
            source: "ASD Source".to_string(),
            location: 1,
            timestamp: "invalid-timestamp".to_string(),
            latitude: 48.8566,
            longitude: 2.3522,
            altitude: Some(120),
            elevation: Some(60),
            gps: Some(1),
            rssi: Some(-85),
            home_lat: Some(48.8566),
            home_lon: Some(2.3522),
            home_height: Some(100.0),
            speed: 15.0,
            heading: 90.0,
            station_name: Some("Station1".to_string()),
            station_latitude: Some(48.8570),
            station_longitude: Some(2.3530),
        };

        let result = asd.fix_tm();
        assert!(result.is_err());
    }
}


