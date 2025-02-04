use serde::{Deserialize, Serialize};

use crate::{Position, DEF_SAC, DEF_SIC};

/// The `Cat129` struct represents a UAS-specific data category introduced in 2019.
/// It is designed to handle data related to drones (civil/military) and is represented
/// as a special category with unique identifiers, aeronautical data, and other drone-related
/// information.
///
/// # Fields
///
/// - `sac`: Source identification, typically set to a default SAC value.
/// - `sic`: Source identification code, typically set to a default SIC value.
/// - `dac`: Destination identification, commonly representing the operator.
/// - `dic`: Destination identification code.
/// - `uas_manufacturer_id`: Manufacturer's identification string.
/// - `uas_model_id`: Drone model's identification string.
/// - `uas_serial`: Unique serial number of the drone.
/// - `uas_reg_country`: Country of drone registration.
/// - `tod`: Time of Day timestamp (in POSIX format).
/// - `position`: A struct representing the geographical position of the drone.
/// - `alt_sea_lvl`: Altitude relative to sea level (in meters or feet).
/// - `alt_gnd_lvl`: Altitude relative to ground level.
/// - `gnss_acc`: GNSS accuracy, a measure of positioning precision.
///
/// - `ground_speed`: Drone speed relative to ground (e.g., in m/s or kt).
/// - `vert_speed`: Vertical speed (e.g., ascent/descent rate in m/s or ft/min).
///
/// The struct provides a default implementation, initializing all fields with
/// predefined default values such as `DEF_SAC` and `DEF_SIC` for identification codes
/// and empty string values for textual fields.
///
/// For more details, refer to the Eurocontrol documentation:
/// <https://www.eurocontrol.int/sites/default/files/2019-06/cat129p29ed12_0.pdf>
///
#[derive(Debug, Serialize, Deserialize)]
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
    pub tod: i64,
    pub position: Position,
    pub alt_sea_lvl: f32,
    pub alt_gnd_lvl: f32,
    pub gnss_acc: f32,
    pub ground_speed: f32,
    pub vert_speed: f32,
}

impl Default for Cat129 {
    fn default() -> Self {
        Cat129 {
            sac: DEF_SAC,
            sic: DEF_SIC,
            dac: DEF_SAC,
            dic: DEF_SIC,
            uas_manufacturer_id: "".to_string(),
            uas_model_id: "".to_string(),
            uas_serial: "".to_string(),
            uas_reg_country: "".to_string(),
            tod: 0i64,
            position: Position::default(),
            alt_sea_lvl: 0.0,
            alt_gnd_lvl: 0.0,
            gnss_acc: 0.0,
            ground_speed: 0.0,
            vert_speed: 0.0,
        }
    }
}
