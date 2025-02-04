use thiserror::Error;

/// This enum defines various authentication-related errors that
/// may occur when interacting with tokens or authentication systems.
///
/// # Variants
///
/// * `BadParam(String)` - Indicates that an invalid or unexpected parameter was provided.
/// * `NoAPIKey` - Signifies that an API key is missing during a required operation.
/// * `Decoding(String)` - Represents an error that occurred while decoding a token.
/// * `HTTP(String)` - Represents an HTTP-related error while retrieving authentication information.
/// * `Retrieval(String)` - Indicates a failure when attempting to retrieve a token.
/// * `Storing(String)` - Error raised when a token cannot be stored properly.
/// * `Expired` - Denotes that a token has expired and is no longer valid.
/// * `Invalid(String)` - Represents an invalid or corrupted token in a specific context.
/// * `Unknown` - A generic error signifying an unknown or unspecified issue.
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
    #[error("Unknown token {0}")]
    TokenError(String),
    #[error("Unknown error.")]
    Unknown,
}
