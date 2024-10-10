use thiserror::Error;

/// Custom error type for tokens, allow us to differentiate between errors.
///
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Bad parameter {0}")]
    BadParam(String),
    #[error("No API Key")]
    NoAPIKey,
    #[error("Decoding token: {0}")]
    Decoding(String),
    #[error("HTTP Error: {0}")]
    HTTP(String),
    #[error("Error retrieving token for {0}")]
    Retrieval(String),
    #[error("Can not store token: {0}")]
    Storing(String),
    #[error("Token expired")]
    Expired,
    #[error("Invalid token in {0}")]
    Invalid(String),
    #[error("Unknown error.")]
    Unknown,
}
