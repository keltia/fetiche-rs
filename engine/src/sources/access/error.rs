use thiserror::Error;

/// Custom error type for the access module, allow us to differentiate between errors.
///
#[derive(Debug, Error)]
pub enum AccessError {
    #[error("Bad configuration parameter: {0}")]
    BadParam(String),
    #[error("No such site {0}")]
    UnknownSite(String),
    #[error("Invalid site {0}")]
    InvalidSite(String),
    #[error("Bad Filter.")]
    BadFilter,
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error("Invalid packet received, can not decode.")]
    BadPacketData,
}

#[derive(Debug, Error)]
pub enum ParamError {
    #[error("No stats actor configured, exiting.")]
    NoStatsActor,
}
