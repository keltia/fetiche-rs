//! Module to deal with different kind of sources we can connect to to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

use std::fmt::Debug;
use std::io::Write;

use anyhow::Result;

use fetiche_formats::{Cat21, Format};
// Re-export these modules for a shorted import path.
//
pub use access::*;
pub use filter::*;
pub use site::*;
pub use sources::*;

mod access;
mod filter;
mod site;
mod sources;

#[macro_use]
mod macros;

/// This trait enables us to manage different ways of connecting and fetching data under
/// a single interface.
///
pub trait Fetchable: Debug {
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String>;
    /// Fetch actual data
    fn fetch(&self, out: &mut dyn Write, token: &str, args: &str) -> Result<()>;
    /// Transform fetched data into Cat21
    fn to_cat21(&self, input: String) -> Result<Vec<Cat21>>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// This trait enables us to manage different ways of connecting and streaming data under
/// a single interface.  The object can connect to a TCP stream or create one by repeatedly calling
/// some API (cf. Opensky).
///
pub trait Streamable: Debug {
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String>;
    /// Stream actual data
    fn stream(&self, out: &mut dyn Write, token: &str, args: &str) -> Result<()>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// Default configuration filename
const CONFIG: &str = "sources.hcl";
const CVERSION: usize = 3;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Relative path to `BASEDIR` for storing auth tokens
const TOKEN_BASE: &str = "tokens";
