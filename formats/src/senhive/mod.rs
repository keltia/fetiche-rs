//! Module for Thales Senhive data feed.
//!

use serde::{Deserialize, Serialize};

pub use alert::*;
pub use fused::*;
pub use state::*;

mod alert;
mod fused;
mod state;

/// Represents geographical coordinates with longitude and latitude.
///
/// This structure is used to encapsulate location data, which includes the
/// longitude (`lon`) and latitude (`lat`) values. These values are represented
/// as `f64` to retain high precision for geospatial calculations.
///
/// # Fields
///
/// * `lon` - The longitude of the coordinate in decimal degrees.
/// * `lat` - The latitude of the coordinate in decimal degrees.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Coordinates {
    pub lon: f64,
    pub lat: f64,
}
