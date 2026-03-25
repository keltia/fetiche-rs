//! Query template rendering and database configuration management.
//!
//! This module provides functionality for managing database variables and rendering SQL query
//! templates. It defines the `DBVars` struct which holds database configuration (plane, drone,
//! and work database names, plus an optional tag) and provides utilities for template-based
//! query generation using the TinyTemplate engine.
//!
//! The main components are:
//! - `DBVars`: A serializable struct containing database names and an optional tag
//! - `load_query()`: Template rendering function that substitutes variables into SQL queries
//! - Context integration methods for extracting database configuration from the application context
//!
//! # Examples
//!
//! ```rust
//! use process_data::cmds::{DBVars, load_query};
//!
//! let dbvars = DBVars {
//!     planedb: "planes_prod".to_string(),
//!     dronedb: "drones_prod".to_string(),
//!     workdb: "work_prod".to_string(),
//!     tag: String::new(),
//! };
//!
//! let query = "SELECT * FROM {planedb}.flights";
//! let rendered = load_query(query, &dbvars).unwrap();
//! assert_eq!(rendered, "SELECT * FROM planes_prod.flights");
//! ```
//!
use crate::runtime::Context;
use serde::Serialize;
use tinytemplate::TinyTemplate;
use tracing::debug;

#[derive(Clone, Debug, Serialize)]
pub struct DBVars {
    pub planedb: String,
    pub dronedb: String,
    pub workdb: String,
    pub tag: String,
}

/// This function instantiates a TinyTemplate with a query template and renders it with variables
/// extracted from the current context.  We usually need the tablespace name for the planes, drones
/// and work databases.
///
/// snprintf(3) for dummies.
///
#[tracing::instrument]
pub fn load_query(q: &str, dbvars: &DBVars) -> eyre::Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("query", q)?;
    let res = tt.render("query", &dbvars)?;
    debug!("q={res}");
    Ok(res)
}

impl DBVars {
    /// Creates a new `DBVars` instance with an updated tag value.
    ///
    /// This method clones the current `DBVars` instance and replaces the tag field
    /// with the provided value, while preserving all other database configuration
    /// fields (planedb, dronedb, workdb).
    ///
    /// # Arguments
    ///
    /// * `tag` - A string slice containing the tag value to set. This is typically
    ///   used to distinguish between different query contexts or temporary tables.
    ///
    /// # Returns
    ///
    /// Returns a new `DBVars` instance with the updated tag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use process_data::cmds::query::DBVars;
    ///
    /// let dbvars = DBVars {
    ///     planedb: "planes_prod".to_string(),
    ///     dronedb: "drones_prod".to_string(),
    ///     workdb: "work_prod".to_string(),
    ///     tag: String::new(),
    /// };
    ///
    /// let tagged = dbvars.tag("_LFPG_20231001");
    /// assert_eq!(tagged.tag, "_LFPG_20231001");
    /// assert_eq!(tagged.planedb, "planes_prod");
    /// ```
    ///
    pub fn tag(&self, tag: &str) -> Self {
        Self {
            planedb: self.planedb.clone(),
            dronedb: self.dronedb.clone(),
            workdb: self.workdb.clone(),
            tag: tag.to_owned(),
        }
    }

    /// Creates a new `DBVars` instance from the application context.
    ///
    /// This function extracts database configuration values from the provided context,
    /// specifically the plane database, drone database, and work database names. These
    /// values are expected to be present in the context's configuration map.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A reference to the application `Context` containing configuration parameters.
    ///
    /// # Returns
    ///
    /// Returns a new `DBVars` instance with the extracted database names and an empty tag.
    ///
    /// # Panics
    ///
    /// This function will panic if any of the required configuration keys (`plane_db`,
    /// `drone_db`, or `work_db`) are not present in the context configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use process_data::runtime::Context;
    /// use process_data::cmds::query::DBVars;
    ///
    /// let ctx = Context {
    ///     config: HashMap::from([
    ///         ("plane_db".to_string(), "planes_tablespace".to_string()),
    ///         ("drone_db".to_string(), "drones_tablespace".to_string()),
    ///         ("work_db".to_string(), "work_tablespace".to_string()),
    ///     ]).into(),
    ///     dry_run: false,
    /// };
    ///
    /// let dbvars = DBVars::from_ctx(&ctx);
    /// assert_eq!(dbvars.planedb, "planes_tablespace");
    /// ```
    ///
    pub fn from_ctx(ctx: &Context) -> Self {
        let planedb = ctx.config.get("plane_db").unwrap();
        let dronedb = ctx.config.get("drone_db").unwrap();
        let workdb = ctx.config.get("work_db").unwrap();
        Self {
            planedb: planedb.to_owned(),
            dronedb: dronedb.to_owned(),
            workdb: workdb.to_owned(),
            tag: String::new(),
        }
    }
}

/// A convenience macro for rendering SQL query templates with database variables.
///
/// This macro provides a shorthand for calling `load_query()` with a query template
/// and a `DBVars` instance. It automatically handles the reference to the `DBVars`
/// and propagates any errors using the `?` operator.
///
/// # Arguments
///
/// * `$q` - A string literal containing the SQL query template with placeholders
///   (e.g., `{planedb}`, `{dronedb}`, `{workdb}`, `{tag}`)
/// * `$val` - An expression that evaluates to a `DBVars` instance containing the
///   database configuration values to substitute into the template
///
/// # Returns
///
/// Returns a `Result<String, eyre::Error>` containing the rendered query on success,
/// or an error if template rendering fails.
///
/// # Errors
///
/// This macro will propagate errors from `load_query()` if:
/// * The template syntax is invalid
/// * Required placeholders in the template don't match fields in `DBVars`
///
/// # Examples
///
/// ```rust
/// # use process_data::cmds::query::DBVars;
/// # use process_data::make_query;
/// # fn main() -> eyre::Result<()> {
/// let dbvars = DBVars {
///     planedb: "planes_prod".to_string(),
///     dronedb: "drones_prod".to_string(),
///     workdb: "work_prod".to_string(),
///     tag: String::new(),
/// };
///
/// let query = make_query!(
///     "SELECT * FROM {planedb}.flights WHERE id > 100",
///     dbvars
/// );
/// assert_eq!(query?, "SELECT * FROM planes_prod.flights WHERE id > 100");
/// # Ok(())
/// # }
/// ```
///
/// Using with tagged queries:
///
/// ```rust
/// # use process_data::cmds::query::DBVars;
/// # use process_data::make_query;
/// # fn main() -> eyre::Result<()> {
/// let dbvars = DBVars {
///     planedb: "planes".to_string(),
///     dronedb: "drones".to_string(),
///     workdb: "work".to_string(),
///     tag: "_temp".to_string(),
/// };
///
/// let query = make_query!(
///     "CREATE TABLE {workdb}.analysis{tag} AS SELECT * FROM {planedb}.data",
///     dbvars
/// );
/// assert_eq!(query?, "CREATE TABLE work.analysis_temp AS SELECT * FROM planes.data");
/// # Ok(())
/// # }
/// ```
///
#[macro_export]
macro_rules! make_query {
    ($q:literal, $val:expr) => {
        crate::cmds::load_query($q, &$val)?
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_query_with_custom_dbvars() {
        let query = "SELECT * FROM {planedb}.flights JOIN {dronedb}.positions ON id = drone_id";
        let dbvars = DBVars {
            planedb: "planes_prod".to_string(),
            dronedb: "drones_prod".to_string(),
            workdb: "work_prod".to_string(),
            tag: String::new(),
        };

        let result = load_query(query, &dbvars).unwrap();
        assert_eq!(
            result,
            "SELECT * FROM planes_prod.flights JOIN drones_prod.positions ON id = drone_id"
        );
    }

    #[test]
    fn test_load_query_with_tag() {
        let query = "CREATE TEMPORARY TABLE temp{tag} AS SELECT * FROM {workdb}.data";
        let dbvars = DBVars {
            planedb: "acute".to_string(),
            dronedb: "acute".to_string(),
            workdb: "acute_work".to_string(),
            tag: "_LFPG_20231001".to_string(),
        };

        let result = load_query(query, &dbvars).unwrap();
        assert_eq!(
            result,
            "CREATE TEMPORARY TABLE temp_LFPG_20231001 AS SELECT * FROM acute_work.data"
        );
    }

    #[test]
    fn test_load_query_invalid_template() {
        let query = "SELECT * FROM {planedb WHERE id = 1";
        let dbvars = DBVars {
            planedb: "acute".to_string(),
            dronedb: "acute".to_string(),
            workdb: "acute_work".to_string(),
            tag: "_LFPG_20231001".to_string(),
        };

        let result = load_query(query, &dbvars);
        assert!(result.is_err());
    }

    #[test]
    fn test_tag_creates_new_instance_with_updated_tag() {
        let dbvars = DBVars {
            planedb: "planes_prod".to_string(),
            dronedb: "drones_prod".to_string(),
            workdb: "work_prod".to_string(),
            tag: String::new(),
        };

        let tagged = dbvars.tag("_LFPG_20231001");
        assert_eq!(tagged.tag, "_LFPG_20231001");
    }

    #[test]
    fn test_tag_preserves_other_fields() {
        let dbvars = DBVars {
            planedb: "planes_prod".to_string(),
            dronedb: "drones_prod".to_string(),
            workdb: "work_prod".to_string(),
            tag: "_old_tag".to_string(),
        };

        let tagged = dbvars.tag("_new_tag");
        assert_eq!(tagged.planedb, "planes_prod");
        assert_eq!(tagged.dronedb, "drones_prod");
        assert_eq!(tagged.workdb, "work_prod");
        assert_eq!(tagged.tag, "_new_tag");
    }

    #[test]
    fn test_tag_with_empty_string() {
        let dbvars = DBVars {
            planedb: "planes_prod".to_string(),
            dronedb: "drones_prod".to_string(),
            workdb: "work_prod".to_string(),
            tag: "_LFPG_20231001".to_string(),
        };

        let tagged = dbvars.tag("");
        assert_eq!(tagged.tag, "");
        assert_eq!(tagged.planedb, "planes_prod");
    }

    #[test]
    fn test_make_query_basic() {
        let dbvars = DBVars {
            planedb: "planes_prod".to_string(),
            dronedb: "drones_prod".to_string(),
            workdb: "work_prod".to_string(),
            tag: String::new(),
        };

        let result: eyre::Result<String> = (|| {
            let query = make_query!("SELECT * FROM {planedb}.flights WHERE id > 100", dbvars);
            Ok(query)
        })();

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "SELECT * FROM planes_prod.flights WHERE id > 100"
        );
    }

    #[test]
    fn test_make_query_with_tag() {
        let dbvars = DBVars {
            planedb: "planes".to_string(),
            dronedb: "drones".to_string(),
            workdb: "work".to_string(),
            tag: "_temp".to_string(),
        };

        let result: eyre::Result<String> = (|| {
            let query = make_query!(
                "CREATE TABLE {workdb}.analysis{tag} AS SELECT * FROM {planedb}.data",
                dbvars
            );
            Ok(query)
        })();

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "CREATE TABLE work.analysis_temp AS SELECT * FROM planes.data"
        );
    }

    #[test]
    fn test_make_query_with_all_variables() {
        let dbvars = DBVars {
            planedb: "planes_prod".to_string(),
            dronedb: "drones_prod".to_string(),
            workdb: "work_prod".to_string(),
            tag: "_v2".to_string(),
        };

        let result: eyre::Result<String> = (|| {
            let query = make_query!(
                "INSERT INTO {workdb}.results{tag} SELECT p.*, d.* FROM {planedb}.data p JOIN {dronedb}.info d",
                dbvars
            );
            Ok(query)
        })();

        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "INSERT INTO work_prod.results_v2 SELECT p.*, d.* FROM planes_prod.data p JOIN drones_prod.info d"
        );
    }

    #[test]
    fn test_make_query_error_propagation() {
        let dbvars = DBVars {
            planedb: "planes".to_string(),
            dronedb: "drones".to_string(),
            workdb: "work".to_string(),
            tag: String::new(),
        };

        let result: eyre::Result<String> = (|| {
            let query = make_query!("SELECT * FROM {planedb WHERE id = 1", dbvars);
            Ok(query)
        })();

        assert!(result.is_err());
    }
}
