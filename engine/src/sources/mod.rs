//! Module to deal with different kind of sources we can connect to fetch data.
//!
//! The different submodules deal with the differences between sources:
//!
//! - authentication (token, API)
//! - fetching data (GET or POST, etc.).
//!
//! The registry stores configured sites, the configuration module loads HCL,
//! and the selection module maps sites to adapters grouped by provider.

pub use adapters::{AccessError, DataError, ParamError};
pub use capability::Capability;
pub use config::{SOURCES_CONFIG, SourcesConfig};
pub use error::AuthError;
pub use registry::Sources;
pub use route::Routes;
pub use selection::{Fetchable, FetchableSource, Streamable, StreamableSource};
pub use site::{DataType, Site};

pub(crate) mod adapters;
mod capability;
mod config;
mod error;
mod registry;
mod route;
mod selection;
mod site;

#[macro_use]
mod macros;
