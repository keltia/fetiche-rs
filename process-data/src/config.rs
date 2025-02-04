//! Configuration module
//!
//! This is where most of the initialisation code lies.  We start the logging process, open
//! the database, etc.
//!
//! Version History:
//!
//! - v1 is for the duckdb-backed database, database is path to the .duckdb file.
//! - v2 is the ClickHouse-backed database, added url/user/password/database
//! - v3 has different sections for parameters
//!

use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use fetiche_common::{IntoConfig, Versioned};
use fetiche_macros::into_configfile;

/// Current version
pub const CVERSION: usize = 3;

/// This module provides the configuration structures and functionalities
/// necessary for initializing the application. It includes definitions for
/// database parameters, distance calculation settings, and integration with
/// external services. The configuration is read from a versioned configuration
/// file to ensure compatibility with different application versions.
///
/// # Current Version
/// The current configuration version is [`CVERSION`] which defines the
/// structure and fields expected in the configuration file.
///
/// # Config File
/// The configuration is stored in a file named [`CONFIG`], and must follow
/// the defined schema to be successfully parsed.
///
/// # Structure
/// - `ProcessConfig`: The main configuration structure holding all settings.
/// - `Database`: Settings related to database connections.
/// - `Distances`: Settings for distance calculation thresholds.
///
/// # Example Configuration
/// ```hcl
/// version = 3
///
/// datalake = "/path/to/datalake"
///
/// db {
///     database = "example_db"
///     url = "http://localhost"
///     user = "admin"
///     password = "password123"
/// }
///
/// distances {
///     threshold = 1852
///     factor = 3
/// }
/// ```
///
#[into_configfile(version = 3, filename = "proces-data.hcl")]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ProcessConfig {
    /// Directory holding the parquet files for the datalake.
    pub datalake: Option<String>,
    /// Section for database parameters.
    pub db: Database,
    /// Section for calculations on distances.
    pub distances: Distances,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Database {
    /// Database name or path.
    pub database: Option<String>,
    /// URL
    pub url: String,
    /// User to connect with
    pub user: Option<String>,
    /// Corresponding password
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Distances {
    /// What we consider a hard safety threshold, in meters.
    pub threshold: u32,
    /// Factor for considering a safety issue, as N times `threshold`
    pub factor: u32,
}

impl Default for Distances {
    fn default() -> Self {
        Self {
            threshold: 1852,
            factor: 3,
        }
    }
}
