use serde::{Deserialize, Serialize};

use crate::{Position, DEF_SAC, DEF_SIC};

/// Cat129 is a special UAS-specific category defined in 2019.
///
/// As the number implies (> 127), it is created to describe a special Civil/Military category,
/// specialised for drones.
///
/// See: <https://www.eurocontrol.int/sites/default/files/2019-06/cat129p29ed12_0.pdf>
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
