//! Module to deal with different kind of sources we can connect to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!

pub use crate::stats::*;

// Re-export these modules for a shorted import path.
//
pub use access::*;
pub use capability::*;
pub use config::{SOURCES_CONFIG, SourcesConfig};
pub use error::*;
pub use registry::Sources;
pub use route::*;
pub use selection::{Fetchable, FetchableSource, Streamable, StreamableSource};
pub use site::*;

mod access;
mod capability;
mod config;
mod error;
mod registry;
mod route;
mod selection;
mod site;

#[macro_use]
mod macros;
