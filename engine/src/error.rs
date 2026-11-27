use thiserror::Error;

/// Token-related errors
///
#[derive(Debug, Error)]
pub enum TokenStatus {
    #[error("Token for {0} not found.")]
    NotFound(String),
    #[error("Unknown token type in {0}")]
    Unknown(String),
    /// The token file exists, but the contents cannot be decoded.
    #[error("malformed token {key} at line {line}, column {column}")]
    Malformed {
        key: String,
        line: usize,
        column: usize,
    },
    /// The storage backend failed to read the token.
    #[error("could not read token {key}")]
    Read {
        key: String,
        #[source]
        source: object_store::Error,
    },
    /// Synchronous token calls must run outside Tokio.
    #[error("use the asynchronous token API inside a Tokio runtime")]
    BlockingCallInRuntime,
}

#[derive(Debug, Error)]
pub enum EngineStatus {
    /// The daemon worker pool must contain at least one runner.
    #[error("workers must be greater than zero")]
    ZeroWorkers,
    /// Scheduler timers require a positive interval.
    #[error("{0} must be greater than zero")]
    ZeroInterval(&'static str),
    #[error("Bad config file version v{0}, need {1}")]
    BadConfigVersion(usize, usize),
    #[error("Can not create directory {0}")]
    CreateDir(String),
    #[error("Can not create link to {0} as {1}")]
    CreateLink(String, String),
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
    #[error("Uninitialised Read")]
    UninitialisedRead,
    #[error("Read input {path:?} exceeds the {max_bytes}-byte limit")]
    ReadTooLarge {
        path: std::path::PathBuf,
        max_bytes: u64,
    },
    #[error("Site {0} is not fetchable")]
    NotFetchable(String),
    #[error("Site {0} is not streamable")]
    NotStreamable(String),
    #[error("Job {0} is running")]
    JobIsRunning(usize),
    #[error("Job {0} is not queued")]
    JobNotQueued(usize),
    #[error("Job {0} is not ready")]
    JobNotReady(usize),
    #[error("Job {0} is not completed")]
    JobNotCompleted(usize),
    #[error("Job {0} is not zombie")]
    JobNotZombie(usize),
    #[error("Job {0} is not created")]
    JobNotCreated(usize),
}

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("Unknown site {0}")]
    UnknownSite(String),
    #[error("Site {0} is not fetchable")]
    NotFetchable(String),
    #[error("Site {0} is not streamable")]
    NotStreamable(String),
    #[error("Invalid job type")]
    InvalidJobType,
}

#[derive(Debug, Error)]
pub enum RunnerError {}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Unrecognized state file format in {0}")]
    UnrecognizedFile(String),
}

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Scheduler is shutting down")]
    ShuttingDown,
    #[error("Job {0} is already queued or running")]
    DuplicateJob(usize),
    #[error("Empty queue.")]
    EmptyQueue,
    #[error("Job {0} is not runnable")]
    JobInWrongState(usize),
    #[error("Unknown job {0}")]
    JobNotFound(usize),
    #[error("Unknown job {0} is not ready to be queued")]
    JobNotReady(usize),
    #[error("Scheduler in the wrong state.")]
    WrongState,
    #[error("Scheduler is not running.")]
    NotRunning,
}

#[derive(Debug, Error)]
pub enum Pipeline {
    #[error("Cannot receive data from previous stage: {0}.")]
    CantReceivePrevious(String),
}

#[derive(Debug, Error)]
pub enum StatsError {
    #[error("Tag {0} not found.")]
    TagNotFound(String),
    #[error("Stats are not initialised.")]
    NotInitialized,
}

#[derive(Debug, Error)]
pub enum StorageError {
    /// A storage interval was invalid or could not be represented in seconds.
    #[error("invalid rotation for storage '{area}': {value}")]
    InvalidRotation { area: String, value: String },
    /// A local storage directory could not be created or verified.
    #[error("storage '{area}' could not use directory {path:?}")]
    Directory {
        area: String,
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Can not create directory tree in {0}")]
    CannotCreateTree(String),
}

#[derive(Debug, Error)]
pub enum WsError {
    #[error("{0} is not a directory")]
    NotADirectory(String),
}

