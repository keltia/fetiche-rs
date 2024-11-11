//! Module for Thales Senhive data feed.
//!

use serde::{Deserialize, Serialize};

pub use alert::*;
pub use fused::*;
pub use state::*;

mod state;
mod alert;
mod fused;

#[derive(Debug, Serialize, Deserialize)]
pub struct Coordinates {
    pub lon: f64,
    pub lat: f64,
}





