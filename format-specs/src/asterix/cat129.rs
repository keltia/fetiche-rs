use crate::Position;

use serde::Serialize;

/// Cat129 is a special UAS-specific category defined in 2019.
///
/// As the number implies (> 127), it is created to describe a special Civil/Military category,
/// specialised for drones.
///
/// See: https://www.eurocontrol.int/sites/default/files/2019-06/cat129p29ed12_0.pdf
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Cat129 {
    /// Source Identification
    pub sac: usize,
    pub sic: usize,
    /// Destination Identification (more or less the operator?)
    pub dac: usize,
    pub dic: usize,
    /// Manufacturer Identification
    pub uas_manufacturer_id: String,
    pub uas_model_id: String,
    pub uas_serial: String,
    pub uas_reg_country: String,
    /// Aeronautical data
    /// tod is number of 1/128s since Midnight
    /// Example:
    /// ```
    /// # use chrono::NaiveDateTime;
    ///
    /// let tod = NaiveDateTime::parse_from_str(&line.timestamp, "%Y-%m-%d %H:%M:%S")
    ///            .unwrap()
    ///            .timestamp();
    /// let tod = 128 * (tod % 86400);
    /// ```
    pub tod: i64,
    pub position: Position,
    pub alt_sea_lvl: f32,
    pub alt_gnd_lvl: f32,
    pub gnss_acc: f32,
    pub ground_speed: f32,
    pub vert_speed: f32,
}
