use std::collections::HashMap;
use std::sync::Arc;

use eyre::Result;
use tracing::{error, info, trace};

use crate::cli::Opts;
use crate::config::FindConfig;
use crate::error::Status;
use crate::NAME;

use fetiche_common::{close_logging, init_logging, ConfigFile, Versioned};

/// Config filename
pub const CONFIG: &str = "airports.hcl";

/// Context holds the shared state and resources for the application.
///
/// This struct contains global settings and resources that are
/// shared across different parts of the application, such as the
/// database connection pool, configuration parameters, and runtime
/// options.
///
/// # Fields
///
/// * `config` - A reference-counted `HashMap` containing configuration parameters.
/// * `dry_run` - A boolean flag indicating whether the application is running
///               in dry-run mode (no side effects).
///
/// # Examples
///
/// ```rust
/// let context = Context {
///     config: Arc::new(HashMap::new()),
///     dry_run: false,
/// };
/// ```
///
#[derive(Clone, Debug)]
pub struct Context {
    /// All configuration parameters
    pub cfg: Arc<HashMap<String, String>>,
    /// Dry run.
    pub dry_run: bool,
}

/// Initializes the runtime environment for the application.
///
/// This function sets up the necessary components such as logging, configuration loading,
/// and database connection pooling. It validates the presence of required parameters such
/// as the database and datalake paths, either from the configuration file or environment
/// variables. In case of missing mandatory parameters or a file version mismatch, it
/// returns an appropriate error.
///
/// # Arguments
///
/// * `opts` - A reference to `Opts` struct with runtime options provided by the user.
///
/// # Returns
///
/// Returns a `Result` that resolves to a `Context` struct containing shared state,
/// database pool, and configuration. An error is returned if critical initialization
/// steps fail (e.g., invalid configuration, missing parameters, or issues with the
/// database connection).
///
/// # Errors
///
/// - `Status::BadFileVersion` if the configuration file version does not match the current version.
/// - `Status::NoDatabase` if the database is not defined in the options or the config file.
/// - `Status::NoDatalake` if the datalake is not defined in the options or the config file.
/// - `Status::NoUrl` if the database URL is missing from the environment or configuration.
///
/// # Examples
///
/// ```rust
/// let opts = Opts {
///     datalake: Some(String::from("/data/lake")),
///     use_telemetry: false,
///     use_tree: false,
///     use_file: None,
/// };
///
/// let context = init_runtime(&opts).await?;
/// ```
///
#[tracing::instrument]
pub async fn init_runtime(opts: &Opts) -> Result<Context> {
    // Initialize logging early
    //
    init_logging(NAME, false, opts.use_tree, opts.use_file.clone())?;
    trace!("Logging initialised.");

    let def = String::from(CONFIG);
    let cfile = ConfigFile::<FindConfig>::load(Some(CONFIG))?;
    let cfg = cfile.inner();

    if cfg.version() != crate::config::CVERSION {
        return Err(Status::BadFileVersion(cfg.version()).into());
    }

    let datalake = match &opts.datalake {
        Some(v) => v,
        None => {
            if let Some(v) = &cfg.datalake {
                v
            } else {
                eprintln!("Error: you must define datalake.");
                return Err(Status::NoDatalake(def).into());
            }
        }
    };

    // No base_url, no fetching, nothing to play with :)
    //
    if cfg.base_url.is_empty() {
        return Err(Status::MissingBaseUrl(def).into());
    }

    if cfg.files.is_empty() {
        return Err(Status::NeedFiles(def).into());
    }

    // Create context
    //
    let ctx = Context {
        cfg: HashMap::from([
            ("base_url".to_string(), cfg.base_url.clone()),
            ("datalake".to_string(), datalake.clone()),
        ])
        .into(),
        dry_run: opts.dry_run,
    };
    Ok(ctx)
}

/// Finish everything.
///
#[tracing::instrument]
pub fn finish_runtime(_ctx: &Context) -> Result<()> {
    close_logging();
    Ok(())
}
