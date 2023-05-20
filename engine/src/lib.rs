//! Library implementing common part of the transformations
//!

use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::Result;

mod fetch;
mod job;
mod task;

pub use fetch::*;
pub use job::*;
pub use task::*;

use fetiche_formats::Format;
use fetiche_sources::Fetchable;

const NAME: &str = env!("CARGO_PKG_NAME");
const EVERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version() -> String {
    format!("{}/{}", NAME, EVERSION)
}

/// Type of task we will need to do
///
#[derive(Debug, Default)]
pub enum Input {
    /// File-based means we need the formats beforehand and a pathname
    ///
    File {
        /// Input formats
        format: Format,
        /// Path of the input file
        path: PathBuf,
    },
    /// Network-based means we need the site name (whose details are taken from the configuration
    /// file.  The `site` is a `Fetchable` object generated from `Config`.
    ///
    Network {
        /// Input formats
        format: Format,
        /// Site itself
        site: Box<dyn Fetchable>,
    },
    #[default]
    Nothing,
}

/// Anything that can be `run()` is runnable.
///
pub trait Runnable: Debug {
    fn run(&self) -> Result<String>;
}
