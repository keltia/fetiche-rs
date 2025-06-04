use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum Status {
    #[error("No database specified anywhere (config: {0}")]
    NoDatabase(String),
    #[error("No datalake specified in {0}")]
    NoDatalake(String),
    #[error("Can't get a connection from pool {0}")]
    ConnectionUnavailable(String),
    #[error("No database URL specified in {0}")]
    NoUrl(String),
    #[error("Bad file version {0}")]
    BadFileVersion(usize),
    #[error("Missing configuration file, use -d or create {0}")]
    MissingConfig(String),
    #[error("Error reading configuration({0})")]
    MissingConfigParameter(String),
}
