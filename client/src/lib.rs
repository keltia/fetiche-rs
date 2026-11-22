mod job;
mod single;

pub use job::*;
pub use single::*;

pub use fetiche_common::Filter;
pub use fetiche_engine::{Freq, JobState};

/// Client signature
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}
