//! Module to deal with different kind of sources we can connect to to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

use std::fmt::{Debug, Display, Formatter};
use std::sync::mpsc::Sender;

use eyre::Result;
use serde::{Deserialize, Serialize};

// Re-export these modules for a shorted import path.
//
pub use access::*;
pub use auth::*;
use fetiche_formats::Format;
pub use filter::*;
pub use route::*;
pub use site::*;
pub use sources::*;

mod access;
mod auth;
mod filter;
mod route;
mod site;
mod sources;

#[macro_use]
mod macros;

#[derive(Clone, Copy, Debug, Default, Deserialize, Ord, PartialOrd, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Capability {
    #[default]
    None = 0,
    Fetch = 1,
    Read = 2,
    Stream = 3,
}

impl Display for Capability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Capability::None => "none",
            Capability::Read => "read",
            Capability::Fetch => "fetch",
            Capability::Stream => "stream",
        };
        write!(f, "{s}")
    }
}

/// This trait enables us to manage different ways of connecting and fetching data under
/// a single interface.
///
pub trait Fetchable: Debug {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String>;
    /// Fetch actual data
    fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> Result<()>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// This trait enables us to manage different ways of connecting and streaming data under
/// a single interface.  The object can connect to a TCP stream or create one by repeatedly calling
/// some API (cf. Opensky).
///
pub trait Streamable: Debug {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    fn authenticate(&self) -> Result<String>;
    /// Stream actual data
    fn stream(&self, out: Sender<String>, token: &str, args: &str) -> Result<()>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// Default configuration filename
const CONFIG: &str = "sources.hcl";
const CVERSION: usize = 4;

#[cfg(unix)]
const BASEDIR: &str = ".config";

/// Relative path to `BASEDIR` for storing auth tokens
const TOKEN_BASE: &str = "tokens";

pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}
