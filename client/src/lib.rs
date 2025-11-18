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
    #[error("cannot create workspace directory {0}")]
    CannotCreate(String),
    #[error("cannot create magic file in {0}")]
    CannotCreateMagic(String),
    #[error("directory {0} is not a workspace, use create()")]
    WsNotInitialized(String),
}
