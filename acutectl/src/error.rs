//! Error module
//!

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Status {
    #[error("Bad file version {0}")]
    BadFileVersion(usize),
    #[error("Missing configuration file, use -d or create {0}")]
    MissingConfig(String),
    #[error("Error reading configuration({0})")]
    MissingConfigParameter(String),
}
