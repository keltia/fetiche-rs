//! Library part of the `acutectl` utility.
//!
//! This library include the code for the different file formats used as input and the different
//! way of fetching data from different sources.  This is written because there are as many ways
//! to authenticate and connect as there are sources more or less.
//!
//! The `client` crate is now used for creating and submitting a job.
//!

/// Re-export
///
pub use cli::*;
pub use cmds::*;
pub use error::*;


mod cli;
mod cmds;
mod error;
