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
    #[error("Site {0} is not Fetchable!")]
    SiteNotFetchable(String),
    #[error("Site {0} is not Streamable!")]
    SiteNotStreamable(String),
    #[error("We need both -B/-E or none")]
    BothOrNone,
    #[error("Can not specify --today and -B/-E")]
    TodayOrBeginEnd,
}
