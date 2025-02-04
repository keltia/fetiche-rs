//! Module to load and process the Aeroscope data coming from ASD and generate
//! CSV data Cat21-like
//!

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{to_feet, to_knots, Cat129, Cat21, Position, TodCalculated};

/// Structure representing the Aeroscope data format.
///
/// The `Aeroscope` struct defines fields corresponding to the Aeroscope CSV output
/// and provides methods for converting this data into other formats, such as `Cat21` or `Cat129`.
///
/// This format includes telemetry and identification information for drones, such as their
/// position, altitude, speed, and type, as well as metadata about their operator and receive time.
///
/// # Fields
///
/// - `id` (`String`): Unique identifier for the Aeroscope record.
/// - `aeroscope_position` (`Position`): The position of the Aeroscope sensor.
/// - `altitude` (`f32`): Altitude of the drone in meters.
/// - `azimuth` (`f32`): Azimuth angle of the drone in degrees.
/// - `coordinate` (`Position`): The drone's geographic position.
/// - `distance` (`f32`): Distance from the Aeroscope sensor to the drone in meters.
/// - `drone_id` (`String`): Identifier for the drone, truncated for privacy.
/// - `drone_type` (`String`): Type of the drone (e.g., Fixed Wing, Multi-rotor).
/// - `flight_id` (`String`): Identifier for the drone's flight.
/// - `home_location` (`Position`): Drone's home location.
/// - `pilot_position` (`Position`): Location of the drone operator.
/// - `receive_date` (`String`): ISO 8601 formatted timestamp indicating when the record was received.
/// - `speed` (`f32`): Ground speed of the drone in meters per second.
///
/// This structure provides compatibility with the ASD site data and is used as
/// an intermediary in the process of generating Cat21-like outputs from Aeroscope-detected drone telemetry.
///
#[derive(Debug, Deserialize, Serialize)]
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
    /// Transforms an `Aeroscope` struct into a `Cat21` struct.
    ///
    /// This implementation converts the data from the `Aeroscope` format
    /// into the `Cat21` format. The following processes are applied:
    ///
    /// - Parsing the `receive_date` field into a `DateTime<Utc>` type and converting
    ///   it to a UNIX timestamp for the `tod` field.
    /// - The `drone_id` field is truncated to the middle section to derive a safe
    ///   and anonymized version, which becomes the `callsign` field.
    ///
    /// All other relevant fields are mapped directly to their counterparts.
    ///
    /// Arbitrary defaults are applied where required:
    /// - `emitter_category` is hardcoded to 13.
    /// - `descriptor_atp` is set to 1.
    /// - `alt_reporting_capability_ft` defaults to 0.
    /// - `target_addr` is set to 623615.
    ///
    /// The default values are arbitrary and taken from the original `aeroscope.sh` script
    /// by Marc Gravis.
    ///
    /// # Example
    ///
    /// ```
    /// use fetiche_formats::{Aeroscope, Cat21, Position};
    ///
    /// let aeroscope = Aeroscope {
    ///     id: "AS12345678".to_string(),
    ///     aeroscope_position: Position::default(),
    ///     altitude: 100.0,
    ///     azimuth: 45.0,
    ///     coordinate: Position { latitude: 48.857, longitude: 2.347 },
    ///     distance: 300.0,
    ///     drone_id: "AB1234567890".to_string(),
    ///     drone_type: "Fixed Wing".to_string(),
    ///     flight_id: "FL56789".to_string(),
    ///     home_location: Position::default(),
    ///     pilot_position: Position::default(),
    ///     receive_date: "2023-10-22T14:00:00Z".to_string(),
    ///     speed: 55.0,
    /// };
    ///
    /// let record: Cat21 = Cat21::from(&aeroscope);
    ///
    /// assert_eq!(record.callsign, "12345678");
    /// assert_eq!(record.alt_geo_ft, 328.084); // altitude converted to feet
    /// assert_eq!(record.groundspeed_kt, 29.6628); // speed converted to knots
    /// assert_eq!(record.track_angle_deg, 45.0); // azimuth mapped as heading
    /// ```
    ///
    /// # Panics
    ///
    /// This function assumes that the `receive_date` field contains a valid ISO 8601
    /// formatted string. An invalid format will cause a panic.
    ///
    /// # Lost Information
    ///
    /// Note that the following fields from the `Aeroscope` struct are not carried over
    /// into the `Cat21` struct:
    /// - `aeroscope_id`
    /// - `aeroscope_position`
    /// - `flight_id`
    /// - `home_location`
    /// - `pilot_position`
    /// ```
    #[tracing::instrument]
    fn from(line: &Aeroscope) -> Self {
        let tod = line.receive_date.parse::<DateTime<Utc>>().unwrap();
        let tod = tod.timestamp();
        let lid = if line.drone_id != "null" {
            line.drone_id[2..10].to_owned()
        } else {
            "null".to_owned()
        };
        Cat21 {
            alt_geo_ft: to_feet(line.altitude),
            pos_lat_deg: line.coordinate.latitude,
            pos_long_deg: line.coordinate.longitude,
            alt_baro_ft: to_feet(line.altitude),
            tod: 128 * (tod % 86400),
            rec_time_posix: tod,
            rec_time_ms: 0,
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
            callsign: lid,
            groundspeed_kt: to_knots(line.speed),
            track_angle_deg: line.azimuth,
            rec_num: 1,
            ..Cat21::default()
        }
    }
}

impl From<&Aeroscope> for Cat129 {
    /// Transforms an `Aeroscope` struct into a `Cat129` struct.
    ///
    /// This implementation extracts and transforms relevant data from the `Aeroscope` format
    /// to the `Cat129` format. The transformation process includes:
    ///
    /// - Parsing the `receive_date` field into a `DateTime<Utc>` and extracting the UNIX timestamp.
    /// - Deriving a truncated `uas_serial` from the `drone_id` for privacy reasons. If the `drone_id`
    ///   is "null", the `uas_serial` will also default to "null".
    ///
    /// Arbitrary defaults are used where information is not available, such as:
    /// - `uas_manufacturer_id` is hardcoded to "DJI".
    /// - `uas_reg_country` defaults to "fr".
    /// - `vert_speed` is defaulted to `1.0`.
    ///
    /// # Panics
    /// This implementation assumes that the `receive_date` field contains a valid timestamp string.
    /// If the parsing fails, the function will panic.
    ///
    /// Example:
    /// ```
    /// use fetiche_formats::{Aeroscope, Cat129, Position};
    ///
    /// let aeroscope = Aeroscope {
    ///     id: "001".to_string(),
    ///     aeroscope_position: Position::default(),
    ///     altitude: 150.0,
    ///     azimuth: 90.0,
    ///     coordinate: Position { latitude: 48.8566, longitude: 2.3522 },
    ///     distance: 200.0,
    ///     drone_id: "AB0012345678".to_string(),
    ///     drone_type: "Quadcopter".to_string(),
    ///     flight_id: "FL12345".to_string(),
    ///     home_location: Position::default(),
    ///     pilot_position: Position::default(),
    ///     receive_date: "2023-10-22T13:45:00Z".to_string(),
    ///     speed: 50.0,
    /// };
    /// let record: Cat129 = Cat129::from(&aeroscope);
    /// assert_eq!(record.uas_serial, "00123456");
    /// ```
    ///
    #[tracing::instrument]
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
            ..Cat129::default()
        }
    }
}
