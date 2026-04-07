use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use crate::cli::Opts;
use crate::config::ProcessConfig;
use crate::error::Status;
use crate::NAME;

use fetiche_common::{close_logging, init_logging, ConfigFile, Versioned};

use klickhouse::{bb8::Pool, Client, ClientOptions, ConnectionManager};
use tracing::{debug, error, info, trace};

/// Config filename
pub const CONFIG: &str = "process-data.hcl";

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
/// * `dbh` - A connection pool to the ClickHouse database.
/// * `wait` - Delay between parallel tasks in milliseconds.
/// * `dry_run` - A boolean flag indicating whether the application is running
///               in dry-run mode (no side effects).
///
/// # Examples
///
/// ```rust
/// let context = Context {
///     config: Arc::new(HashMap::new()),
///     dbh: db_pool,
///     pool_size: 10,
///     wait: 100,
///     dry_run: false,
/// };
///
/// // Use context for database operations
/// let client = context.db().await;
/// ```
///
#[derive(Clone)]
pub struct Context {
    /// All configuration parameters
    pub config: Arc<HashMap<String, String>>,
    /// Database Client.
    pub dbh: Arc<Client>,
    /// Threshold for batching.
    pub batch_size: usize,
    /// Dry run
    pub dry_run: bool,
}

impl Context {
    /// Returns a `Client` from the database connection pool.
    ///
    /// This method retrieves an available ClickHouse client from the connection pool.
    /// If the pool is exhausted or unavailable, an appropriate error is logged and returned.
    ///
    /// # Arguments
    ///
    /// * `self` - A reference to the `Context` struct providing access to the connection pool.
    ///
    /// # Returns
    ///
    /// Returns an active `Client` for interacting with the ClickHouse database.
    ///
    /// # Errors
    ///
    /// - Returns `Status::ConnectionUnavailable` if no connection is available from the pool.
    /// - Panics if the result from the connection pool cannot be unwrapped (typically indicates a critical failure).
    ///
    /// # Examples
    ///
    /// ```rust
    /// let client = context.db().await;
    /// ```
    ///
    #[tracing::instrument(skip(self))]
    pub async fn db(&self) -> Arc<Client> {
        let client = self.dbh.clone();
        client
    }

    /// Finalize the runtime environment and ensure cleanup.
    ///
    /// This method is responsible for performing any necessary cleanup or
    /// finalization operations before the application exits.
    ///
    /// # Returns
    ///
    /// Returns a `Result<()>` indicating whether the finalization
    /// completed successfully.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Finalize runtime components
    /// context.finish()?;
    /// ```
    ///
    /// # Errors
    ///
    /// This function currently does not return any errors explicitly,
    /// but future updates might include additional error conditions.
    ///
    #[tracing::instrument(skip(self))]
    pub fn finish(&self) -> eyre::Result<()> {
        Ok(())
    }
}

impl Debug for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("config", &self.config)
            .field("dbh", &String::from("Clickhouse client"))
            .field("dry_run", &self.dry_run)
            .finish()
    }
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
///     database: None,
///     datalake: Some(String::from("/data/lake")),
///     use_telemetry: false,
///     use_tree: false,
///     use_file: None,
///     pool_size: 10,
///     wait: 100,
///     dry_run: false,
/// };
///
/// let context = init_runtime(&opts).await?;
/// ```
///
#[tracing::instrument]
pub async fn init_runtime(opts: &Opts) -> eyre::Result<Context> {
    // Initialise logging early
    //
    init_logging(
        NAME,
        opts.use_telemetry,
        opts.use_tree,
        opts.use_file.clone(),
    )?;
    trace!("Logging initialised.");

    let def = String::from(CONFIG);
    let cfile = ConfigFile::<ProcessConfig>::load(Some(CONFIG))?;
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

    // We must operate on a database, so we need a profile.
    //
    if cfg.profiles.is_empty() {
        return Err(Status::MissingConfigParameter("profiles".into()).into());
    }

    // Check what profiles are available
    // At this point, it is either $CLICKHOUSE_PROFILE or "default"
    //
    // Priority:
    // - CLI option, if present,
    // - Environment variable, if present, defaults to "default"
    //
    let profile_name = opts
        .profile
        .clone()
        .unwrap_or_else(|| std::env::var("CLICKHOUSE_PROFILE").unwrap_or("default".into()));

    let profile = match cfg.profiles.get(&profile_name) {
        Some(p) => p,
        None => {
            return Err(Status::MissingProfile(profile_name.into()).into());
        }
    };

    trace!("Using profile {} = {}", profile_name, profile);

    // Retrieve some values out of the environment.
    // If not, values are extracted from the configuration file.
    //
    // Allow database names to be overridden on the command line
    //
    let user = std::env::var("CLICKHOUSE_USER").unwrap_or(cfg.db.user.clone().unwrap());
    let pass = std::env::var("CLICKHOUSE_PASSWD").unwrap_or(cfg.db.password.clone().unwrap());
    let endpoint = std::env::var("KLICKHOUSE_URL").unwrap_or(cfg.db.url.clone());

    // URL is mandatory, either in environment or in the config file.
    //
    if endpoint.is_empty() {
        error!("DB URL not defined, exiting!");
        return Err(Status::NoUrl(def).into());
    }

    info!("Connecting to {} @ {}", profile.work_db, endpoint);
    trace!("Creating connection pool");

    let client = Client::connect(
        endpoint.clone(),
        ClientOptions {
            username: user.clone(),
            password: pass.clone(),
            default_database: profile.work_db.clone(),
            ..Default::default()
        },
    )
    .await?;

    // Extract the threshold parameter, which define the size of batches
    //
    let threshold = opts.batch_size;

    let ctx = Context {
        config: HashMap::from([
            ("datalake".to_string(), datalake.clone()),
            ("profile".into(), profile_name.clone()),
            ("planedb".into(), profile.plane_db.clone()),
            ("dronedb".into(), profile.drone_db.clone()),
            ("workdb".into(), profile.work_db.clone()),
        ])
        .into(),
        dbh: Arc::new(client),
        batch_size: threshold,
        dry_run: opts.dry_run,
    };
    debug!("{:?}", &ctx.config);
    Ok(ctx)
}

/// Finish everything.
///
#[tracing::instrument]
pub fn finish_runtime(_ctx: &Context) -> eyre::Result<()> {
    close_logging();
    Ok(())
}
