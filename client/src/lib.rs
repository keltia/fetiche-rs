mod grpc;
mod job;
mod local;
mod single;

pub use job::*;
pub use local::*;
pub use single::*;

// Re-export engine stuff.
pub use fetiche_engine::{Filter, Freq, JobState};

/// Client signature
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

