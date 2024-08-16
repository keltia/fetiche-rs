//! Configuration module
//!
//! This is where most of the initialisation code lies.  We start the logging process, open
//! the database, etc.
//!

use std::collections::HashMap;
use std::sync::Arc;

use clickhouse::Client;
use eyre::Result;
use tracing::{error, info, trace};

use fetiche_common::{close_logging, init_logging};
pub use io::*;

use crate::cli::Opts;
use crate::error::Status;
use crate::NAME;

mod io;
/// This holds our context, meaning common stuff
///
#[derive(Clone)]
pub struct Context {
    /// All configuration parameters
    pub config: Arc<HashMap<String, String>>,
    /// Database Client.
    dbh: Client,
    /// Delay between parallel tasks
    pub wait: u64,
    /// Dry run
    pub dry_run: bool,
}

impl Context {
    #[tracing::instrument(skip(self))]
    pub fn db(&self) -> Client {
        self.dbh.clone()
    }

    #[tracing::instrument(skip(self))]
    pub fn finish(&self) -> Result<()> {
        Ok(())
    }
}

/// Connect to database and load the extensions.
///
#[tracing::instrument]
pub fn init_runtime(opts: &Opts) -> Result<Context> {
    // Initialise logging early
    //
    init_logging(NAME, opts.use_telemetry)?;
    trace!("Logging initialised.");

    // We must operate on a database.
    //
    let def = ConfigFile::default_file().to_string_lossy().to_string();
    let cnf = opts.config.clone().unwrap_or(def.clone());
    let cfg = ConfigFile::load(&cnf)?;

    if opts.database.is_none() && cfg.database.is_none() {
        return Err(Status::NoDatabase(def).into());
    }

    if cfg.datalake.is_none() {
        eprintln!("Error: you must define datalake.");
        return Err(Status::NoDatalake(def).into());
    }

    // Extract parameters
    //
    let datalake = cfg.datalake.unwrap();
    let name = std::env::var("CLICKHOUSE_DB")
        .unwrap_or(opts.database.clone().unwrap_or(cfg.database.unwrap()));
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
    let dbh = Client::default()
        .with_url(endpoint.clone())
        .with_database(&name)
        .with_user(&user)
        .with_password(pass);

    let ctx = Context {
        config: HashMap::from([
            ("url".to_string(), endpoint.clone()),
            ("database".to_string(), name.clone()),
            ("datalake".to_string(), datalake.clone()),
            ("username".to_string(), user.clone()),
        ])
        .into(),
        dbh,
        wait: opts.wait,
        dry_run: opts.dry_run,
    };
    Ok(ctx)
}

/// Finish everything.
///
#[tracing::instrument]
pub fn finish_runtime() -> Result<()> {
    close_logging();
    Ok(())
}
