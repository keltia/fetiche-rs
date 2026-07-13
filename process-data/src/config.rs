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
//! - v4 added the plane parameter into the distances section
//! - v5 splits database into plane_db, drone_db a,d work_db.
//! - v6 added the profiles section.
//!

use std::collections::HashMap;
use std::fmt::{Debug, Display};

use serde::{Deserialize, Serialize};

use fetiche_common::{IntoConfig, Versioned};
use fetiche_macros::into_configfile;

/// Current version
pub const CVERSION: usize = 6;

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
/// version = 6
///
/// datalake = "/path/to/datalake"
/// airports = "/path/to/airports.parquet"
///
/// db {
///     url = "https://localhost:8443"
///     user = "admin"
///     password = "password123"
/// }
///
/// profiles {
///   "prod" = {
///     // fetch plane data from this namespace
///     "plane_db" = "prod_planes"
///     // fetch drone data from this namespace
///     "drone_db" = "prod_drones"
///     // working tables will be in this namespace
///     "work_db" = "prod_work"
///   }
///   "dev" = {
///     "plane_db" = "dev_planes"
///     "drone_db" = "dev_drones"
///     "work_db"  = "dev_work"
///   }
/// }
/// distances {
///     threshold = 1852
///     factor = 3
///     plane = 70
/// }
/// ```
///
#[into_configfile(version = 6, filename = "proces-data.hcl")]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ProcessConfig {
    /// Path to the datalake.
    pub datalake: Option<String>,
    /// Path to the "airports.parquet" file.
    pub airports: Option<String>,
    /// Section for database parameters.
    pub db: Database,
    /// Section for calculations on distances.
    pub distances: Distances,
    /// Section for profiles.
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Database {
    /// URL
    pub url: String,
    /// User to connect with
    pub user: Option<String>,
    /// Corresponding password
    pub password: Option<String>,
}

/// A "profile" is a triplet containing the different namespaces used for the database.
///
#[derive(Debug, Deserialize, Serialize, Eq, Ord, PartialOrd, PartialEq)]
pub struct Profile {
    /// Database holding the plane data.
    pub plane_db: String,
    /// Database holding the drone data.
    pub drone_db: String,
    /// Database holding the working tables.
    pub work_db: String,
}

impl Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ plane_db: {:>10}, drone_db: {:>10}, work_db: {:>10} }}", self.plane_db, self.drone_db, self.work_db)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Distances {
    /// What we consider a hard safety threshold, in meters.
    pub threshold: u32,
    /// Factor for considering a safety issue, as N times `threshold`
    pub factor: u32,
    /// Plane radius, in meters.
    pub plane: u32,
}

impl Default for Distances {
    fn default() -> Self {
        Self {
            threshold: 1852,
            factor: 3,
            plane: 70,
        }
    }
}
