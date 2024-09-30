use thiserror::Error;

/// Token-related errors
///
#[derive(Debug, Error)]
pub enum TokenStatus {
    #[error("Token for {0} not found.")]
    NotFound(String),
    #[error("Unknown token type in {0}")]
    Unknown(String),
}

#[derive(Debug, Error)]
pub enum EngineStatus {
    #[error("Bad config file version v{0}, need {1}")]
    BadConfigVersion(usize, usize),
    #[error("Can not create directory {0}")]
    CreateDir(String),
    #[error("Can not create link to {0} as {1}")]
    CreateLink(String, String),
    #[error("Empty task list.")]
    EmptyTaskList,
    #[error("Site not found.")]
    NoSiteDefined,
    #[error("First task must be Producer.")]
    NoFirstProducer,
    #[error("Last task must be Filter/Producer.")]
    NoLastConsumer,
    #[error("No path defined for Store.")]
    NoPathDefined,
    #[error("Only Asd to Parquet for now.")]
    OnlyAsdToParquet,
    #[error("Can not remove symlink {0}")]
    RemoveLink(String),
    #[error("Unknown token {0}")]
    TokenError(String),
    #[error("Uninitialised Read")]
    UninitialisedRead,
}
