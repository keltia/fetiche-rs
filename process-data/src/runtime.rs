use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use klickhouse::bb8::Pool;
use klickhouse::{bb8, Client, ClientOptions, ConnectionManager};
use tracing::{error, info, trace};

use crate::cli::Opts;
use crate::config::ProcessConfig;
use crate::error::Status;
use crate::NAME;
use fetiche_common::{close_logging, init_logging, ConfigFile, Versioned};

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
/// * `pool_size` - Maximum number of connections allowed in the database pool.
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
    pub dbh: Pool<ConnectionManager>,
    /// Current DB pool size.
    pub pool_size: usize,
    /// Delay between parallel tasks
    pub wait: u64,
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
    pub async fn db(&self) -> Client {
        let client = self
            .dbh
            .get()
            .await
            .map_err(|e| Status::ConnectionUnavailable(e.to_string()))
            .unwrap();
        client.clone()
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
            .field("wait", &self.wait)
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

    // We must operate on a database.
    //
    if opts.database.is_none() && cfg.db.database.is_none() {
        return Err(Status::NoDatabase(def).into());
    }

    // Get some sane values
    //
    let database = match &opts.database {
        Some(v) => v,
        None => {
            if let Some(v) = &cfg.db.database {
                v
            } else {
                eprintln!("Error: you must define database.");
                return Err(Status::NoDatabase(def).into());
            }
        }
    };
    // Extract parameters
    //
    // Allow database to be overridden on command line
    //
    let name = std::env::var("CLICKHOUSE_DB")
        .unwrap_or(opts.database.clone().unwrap_or(database.to_string()));
    let user = std::env::var("CLICKHOUSE_USER").unwrap_or(cfg.db.user.clone().unwrap());
    let pass = std::env::var("CLICKHOUSE_PASSWD").unwrap_or(cfg.db.password.clone().unwrap());
    let endpoint = std::env::var("KLICKHOUSE_URL").unwrap_or(cfg.db.url.clone());

    // URL is mandatory, either in environment or in the config file.
    //
    if endpoint.is_empty() {
        error!("DB URL not defined, exiting!");
        return Err(Status::NoUrl(def).into());
    }

    info!("Connecting to {} @ {}", name, endpoint);
    trace!("Creating connection pool");

    let manager = ConnectionManager::new(
        endpoint.clone(),
        ClientOptions {
            username: user.clone(),
            password: pass.clone(),
            default_database: name.clone(),
            ..Default::default()
        },
    )
        .await?;

    let pool_size = opts.pool_size;
    let pool = bb8::Pool::builder()
        .retry_connection(true)
        .max_size(pool_size as u32)
        .build(manager)
        .await?;

    // Extract the threshold parameter, which define the minimal safety
    // distance.
    //
    let threshold = cfg.distances.threshold;
    let factor = cfg.distances.factor;

    let ctx = Context {
        config: HashMap::from([
            ("url".to_string(), endpoint.clone()),
            ("database".to_string(), name.clone()),
            ("datalake".to_string(), datalake.clone()),
            ("username".to_string(), user.clone()),
            ("threshold".to_string(), threshold.to_string()),
            ("factor".to_string(), factor.to_string())
        ])
            .into(),
        dbh: pool.clone(),
        pool_size,
        wait: opts.wait,
        dry_run: opts.dry_run,
    };
    Ok(ctx)
}

/// Finish everything.
///
#[tracing::instrument]
pub fn finish_runtime(_ctx: &Context) -> eyre::Result<()> {
    close_logging();
    Ok(())
}

