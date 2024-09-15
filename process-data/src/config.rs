//! Configuration module
//!
//! This is where most of the initialisation code lies.  We start the logging process, open
//! the database, etc.
//!
//! Version History:
//!
//! - v1 is for the duckdb-backed database, database is path to the .duckdb file.
//! - v2 is the ClickHouse-backed database, added url/user/password/database
//!

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use eyre::Result;
use klickhouse::bb8::Pool;
use klickhouse::{bb8, Client, ClientOptions, ConnectionManager};
use serde::{Deserialize, Serialize};
use tracing::{error, info, trace};

use fetiche_common::{close_logging, init_logging, ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;

use crate::cli::Opts;
use crate::error::Status;
use crate::NAME;

/// Config filename
const CONFIG: &str = "process-data.hcl";

/// Current version
const CVERSION: usize = 2;

/// Configuration for the CLI tool
///
#[into_configfile(version = 2, filename = "proces-data.hcl")]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ProcessConfig {
    /// Database name or path.
    pub database: Option<String>,
    /// Directory holding the parquet files for the datalake.
    pub datalake: Option<String>,
    /// URL
    pub url: String,
    /// User to connect with
    pub user: Option<String>,
    /// Corresponding password
    pub password: Option<String>,
}

/// This holds our context, meaning common stuff
///
#[derive(Clone)]
pub struct Context {
    /// All configuration parameters
    pub config: Arc<HashMap<String, String>>,
    /// Database Client.
    dbh: Pool<ConnectionManager>,
    /// Delay between parallel tasks
    pub wait: u64,
    /// Dry run
    pub dry_run: bool,
}

impl Context {
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

    #[tracing::instrument(skip(self))]
    pub fn finish(&self) -> Result<()> {
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

/// Connect to database and load the extensions.
///
#[tracing::instrument]
pub async fn init_runtime(opts: &Opts) -> Result<Context> {
    // Initialise logging early
    //
    init_logging(
        NAME,
        opts.use_telemetry,
        opts.use_tree,
        opts.use_file.clone(),
    )?;
    trace!("Logging initialised.");

    // We must operate on a database.
    //
    let def = String::from(CONFIG);
    let cfile = ConfigFile::<ProcessConfig>::load(Some(CONFIG))?;
    let cfg = cfile.inner();

    if cfg.version() != CVERSION {
        return Err(Status::BadFileVersion(cfg.version()).into());
    }

    if opts.database.is_none() && cfg.database.is_none() {
        return Err(Status::NoDatabase(def).into());
    }

    // Get some sane values
    //
    let database = match &opts.database {
        Some(v) => v,
        None => {
            if let Some(v) = &cfg.database {
                v
            } else {
                eprintln!("Error: you must define database.");
                return Err(Status::NoDatabase(def).into());
            }
        }
    };
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

    // Extract parameters
    //
    // Allow database to be overridden on command line
    //
    let name = std::env::var("CLICKHOUSE_DB")
        .unwrap_or(opts.database.clone().unwrap_or(database.to_string()));
    let user = std::env::var("CLICKHOUSE_USER").unwrap_or(cfg.user.clone().unwrap());
    let pass = std::env::var("CLICKHOUSE_PASSWD").unwrap_or(cfg.password.clone().unwrap());
    let endpoint = std::env::var("CLICKHOUSE_URL").unwrap_or(cfg.url.clone());

    // URL is mandatory, either in environment or in the config file.
    //
    if endpoint.is_empty() {
        error!("DB URL not defined, exiting!");
        return Err(Status::NoUrl(def).into());
    }

    info!("Connecting to {} @ {}", name, endpoint);
    trace!("Creating connection pool");

    let manager = ConnectionManager::new(
        endpoint,
        ClientOptions {
            username: user,
            password: pass,
            default_database: name,
        },
    )
    .await?;

    let pool = bb8::Pool::builder().max_size(8).build(manager).await?;

    let ctx = Context {
        config: HashMap::from([
            ("url".to_string(), endpoint.clone()),
            ("database".to_string(), name.clone()),
            ("datalake".to_string(), datalake.clone()),
            ("username".to_string(), user.clone()),
        ])
        .into(),
        dbh: pool.clone(),
        wait: opts.wait,
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
