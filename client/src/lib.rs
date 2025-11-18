use thiserror::Error;

mod job;
mod local;
mod single;
mod workspace;

pub use job::*;
pub use local::*;
pub use single::*;
pub use workspace::*;

// Re-export engine stuff.
pub use fetiche_engine::{Filter, Freq, JobState};

/// Client signature
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}


#[derive(Debug, Error)]
pub enum WsError {
    #[error("{0} is not a directory")]
    NotADirectory(String),
}
