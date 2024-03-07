//! This library is there to share some common code amongst all fetiche modules.
//!

mod container;
mod dateopts;
mod daterange;
mod location;
mod macros;

use clap::{crate_name, crate_version};
pub use container::*;
pub use dateopts::*;
pub use daterange::*;
pub use location::*;

const NAME: &str = crate_name!();
const VERSION: &str = crate_version!();

pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}
