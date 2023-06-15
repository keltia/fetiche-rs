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
#[derive(Clone, Debug, Default)]
pub enum IO {
    /// Consumer
    In,
    /// Producer
    Out,
    /// Both (filter)
    #[default]
    InOut,
    /// Cache (filter)
    Cache,
}
