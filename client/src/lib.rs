//! In-process client for the Fetiche engine.
//!
//! Define a pipeline with [`JobText::builder`] or [`JobBuilder`], then use
//! [`EngineSingle`] to parse and run it. The client also exposes the engine's
//! configuration paths and human-readable listings of supported capabilities.

mod job;
mod single;

pub use job::*;
pub use single::*;

pub use fetiche_common::Filter;
pub use fetiche_engine::{Freq, JobState};

/// Client signature
///
/// Returns the client crate name and version in `name/version` form.
///
/// See [`EngineSingle::version`] for the engine and its component versions.
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}
