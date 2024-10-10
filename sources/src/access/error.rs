use thiserror::Error;

/// Custom error type for the access module, allow us to differentiate between errors.
///
#[derive(Error)]
pub enum AccessError {
    #[error("Bad configuration parameter: {0}")]
    BadParam(String),
}
