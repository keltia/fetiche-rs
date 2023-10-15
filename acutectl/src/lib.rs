//! Library part of the `acutectl` utility.
//!
//! This library include the code for the different file formats used as input and the different
//! way of fetching data from different sources.  This is written because there are as many ways
//! to authenticate and connect as there are sources more or less.
//!
//! The different formats are in the `formats` crate and the sources' parameters in the
//! `sources` crate.
//!
//! The `fetiche-engine` crate is now used for the tasks/jobs.
//!

/// Re-export
///
pub use cli::*;
pub use cmds::*;
pub use config::*;
pub use engine::*;

mod cli;
mod cmds;
mod config;
mod engine;
