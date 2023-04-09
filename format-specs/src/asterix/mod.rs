//! Asterix specific module
//!
//! Regroup the various pseudo-categories we can use.
//!

mod cat129;
mod cat21;

pub use cat129::*;
pub use cat21::*;

/// Default SAC: France
pub const DEF_SAC: usize = 8;
/// Default SIC
pub const DEF_SIC: usize = 200;
