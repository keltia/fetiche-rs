use thiserror::Error;

#[derive(Debug, Error)]
pub enum Fetiched {
    #[error("Can not read configuration in {0}.")]
    UnreadableConfig(String),
    #[error("Can not fetch keys from {0}.")]
    CannotFetchKeys(String),
    #[error("Can not detach myself: {0}")]
    CantDetach(String),
    #[error("PID file {0} already exists, please check.")]
    PidExists(String),
}