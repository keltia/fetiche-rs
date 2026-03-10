//! Configuration module
//!
//! This is where most of the initialisation code lies.  We start the logging process, open
//! the database, etc.
//!
//! Version History:
//!
//! - v1: initial version.
//!

use serde::{Deserialize, Serialize};

use fetiche_common::{IntoConfig, Versioned};
use fetiche_macros::into_configfile;

/// Current version
pub const CVERSION: usize = 1;

/// This module provides the configuration structures and functionalities
/// necessary for initialising the application. It includes definitions for
/// database parameters, distance calculation settings, and integration with
/// external services. The configuration is read from a versioned configuration
/// file to ensure compatibility with different application versions.
///
/// # Current Version
/// The current configuration version is [`CVERSION`] which defines the
/// structure and fields expected in the configuration file.
///
/// # Example Configuration
/// ```hcl
/// version = 1
///
/// datalake = "/path/to/datalake"
/// base_url = "https://davidmegginson.github.io/ourairports-data/"
/// file = "airports.csv"
/// ```
///
#[into_configfile(version = 1, filename = "airports.hcl")]
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FindConfig {
    /// Directory holding the parquet files in the datalake.
    pub datalake: Option<String>,
    /// Base URL for all downloads.
    pub base_url: String,
    /// Files to be fetched.
    pub sources: Vec<String>,
}
