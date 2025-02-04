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
pub enum RunnerError {}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("Empty queue.")]
    EmptyQueue,
    #[error("Job {0} is not runnable")]
    JobInWrongState(usize),
    #[error("Unknown job {0}")]
    JobNotFound(usize),
    #[error("Unknown job {0} is not ready to be queued")]
    JobNotReady(usize),
}

#[derive(Debug, Error)]
pub enum SchedulerError {
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
