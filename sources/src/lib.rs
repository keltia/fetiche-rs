//! Module to deal with different kind of sources we can connect to to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

pub mod config;
pub mod filter;
pub mod s;
pub mod site;

#[macro_use]
mod macros;

use std::fmt::Debug;

use anyhow::Result;

use format_specs::output::Cat21;
use format_specs::Format;

/// This trait enables us to manage different ways of connecting and fetching data under
/// a single interface.
///
pub trait Fetchable: Debug {
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String>;
    /// Fetch actual data
    fn fetch(&self, token: &str, args: &str) -> Result<String>;
    /// Transform fetched data into Cat21
    fn process(&self, input: String) -> Result<Vec<Cat21>>;
    /// Returns the input format-specs
    fn format(&self) -> Format;
}
