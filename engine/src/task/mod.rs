//! Regroup all available task/commands
//!

pub use common::*;
pub use convert::*;
pub use fetch::*;
pub use read::*;
pub use stream::*;

pub mod common;
pub mod convert;
pub mod fetch;
pub mod read;
pub mod stream;

/// Task I/O characteristics
///
/// The main principle being that a consumer should not be first in a job queue
/// just like an Out one should not be last.
///
#[derive(Clone, Debug, Default)]
pub enum IO {
    /// Consumer (no output or different like file)
    In,
    /// Producer (discard input)
    Out,
    /// Both (filter)
    #[default]
    InOut,
    /// Cache (filter)
    Cache,
}
