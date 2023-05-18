//! Library part of the `acutectl` utility.
//!
//! This library include the code for the different file formats used as input and the different
//! way of fetching data from different sources.  This is written because there are as many ways
//! to authenticate and connect as there are sources more or less.
//!
//! The different formats are in the `format-specs` crate and the sources' parameters in the
//! `sources` crate.
//!
//! Include Task-related code.
//!
//! A task is a job that we have to perform.  It can be either a file-based or a network-based one.
//! We have a set of methods to add parameter and configure the task then we need to call `run()`
//! to execute it.
//!

use std::path::PathBuf;

use fetiche_sources::Fetchable;
use format_specs::Format;

/// Re-export
///
pub use cli::*;
pub use cmds::*;
pub use task::*;

mod cli;
mod cmds;
mod task;

/// Type of task we will need to do
///
#[derive(Debug, Default)]
pub enum Input {
    /// File-based means we need the format-specs beforehand and a pathname
    ///
    File {
        /// Input format-specs
        format: Format,
        /// Path of the input file
        path: PathBuf,
    },
    /// Network-based means we need the site name (whose details are taken from the configuration
    /// file.  The `site` is a `Fetchable` object generated from `Config`.
    ///
    Network {
        /// Input format-specs
        format: Format,
        /// Site itself
        site: Box<dyn Fetchable>,
    },
    #[default]
    Nothing,
}
