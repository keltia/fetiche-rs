//! Asterix specific module
//!
//! Regroup the various pseudo-categories we can use.
//!
//! FIXME: the whole crate is obsolete, and will disappear at some point.
//!
//! `cat129` might stay if there is enough interest.
//!

mod adsb;
mod cat129;
mod cat21;

pub use adsb::*;
pub use cat129::*;
pub use cat21::*;

/// Default SAC: France
pub const DEF_SAC: usize = 8;
/// Default SIC
pub const DEF_SIC: usize = 200;

/// For privacy reasons, we truncate the drone ID value to something not unique
///
#[cfg(feature = "privacy")]
pub fn get_drone_id(id: &str) -> String {
    id[2..10].to_owned()
}

#[cfg(not(feature = "privacy"))]
pub fn get_drone_id(id: &str) -> String {
    id.to_owned()
}
