//! Library implementing common part of the transformations
//!

use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::Result;

mod common;
mod fetch;
mod job;
mod parse;

pub use common::*;
pub use fetch::*;
pub use job::*;
pub use parse::*;

use fetiche_formats::Format;
use fetiche_sources::{Fetchable, Sources};

const NAME: &str = env!("CARGO_PKG_NAME");
const EVERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version() -> String {
    format!("{}/{}", NAME, EVERSION)
}

/// Main `Engine` struct that hold the sources and everything needed to perform
///
#[derive(Debug)]
pub struct Engine {
    /// Sources
    pub sources: Sources,
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
