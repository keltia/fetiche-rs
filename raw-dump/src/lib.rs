//! Library part of the `raw-dump` utility.
//!
//! This library include the code for the different file formats used as input and the different
//! way of fetching data from different sources.  This is written because there are as many ways
//! to authenticate and connect as there are sources more or less.
//!
//! The different formats are in the `format-specs` crate and the sources' parameters in the `site` crate.
//!
//! Include Task-related code.
//!
//! A task is a job that we have to perform.  It can be either a file-based or a network-based one.
//! We have a set of methods to add parameter and configure the task then we need to call `run()`
//! to execute it.
//!

mod cli;
mod fetch;
mod task;

use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::debug;

use format_specs::{Cat21, Format};

use sources::{Fetchable, Filter};

/// Re-export
///
pub use cli::*;
pub use fetch::*;
pub use task::*;

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
