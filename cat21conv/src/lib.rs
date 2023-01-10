//! Library part of the Cat21 converter
//!
//! This library include the code for the different file formats used as input and the different
//! way of fetching data from different sites.  This is written because there are as many ways
//! to authenticate and connect as there are sites more or less.
//!
//! The different formats are in the `format-specs` crate and the sites' parameters in the `site` crate.
//!

use clap::{crate_name, crate_version};

pub mod filter;
pub mod site;
pub mod task;

pub(crate) const VERSION: &str = crate_version!();
pub(crate) const NAME: &str = crate_name!();

/// Returns the library version
///
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}
