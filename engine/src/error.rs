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
