mod io;

pub use io::*;
use std::collections::HashMap;

use duckdb::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::cli::Opts;
use crate::cmds::Status;

use crate::NAME;
use eyre::Result;
use tracing::{info, trace};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

/// This holds our context, meaning common stuff
///
#[derive(Debug)]
pub struct Context {
    /// All configuration parameters
    pub config: Arc<HashMap<String, String>>,
    /// Database connection.
    dbh: Arc<Connection>,
}

impl Context {
    pub fn db(&self) -> Arc<Connection> {
        self.dbh.clone()
    }

    pub fn finish(&self) -> Result<()> {
        let dbh = self.dbh.as_ref().try_clone()?;
        let _ = dbh.close();
        Ok(())
    }
}

/// Connect to database and load the extensions.
///
#[tracing::instrument]
pub fn init_runtime(opts: &Opts) -> Result<Context> {
    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_ansi(true)
        .with_span_retrace(true)
        .with_span_modes(true)
        .with_targets(true)
        .with_verbose_entry(true)
        .with_verbose_exit(true)
        .with_higher_precision(true)
        .with_bracketed_fields(true);

    // Setup Open Telemetry with Jaeger
    //
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_auto_split_batch(true)
        .with_max_packet_size(9_216)
        .with_service_name(NAME)
        .install_simple()?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .with(telemetry)
        .init();
    trace!("Logging initialised.");

    // We must operate on a database.
    //
    let cnf = opts.config.clone();
    let cfg = ConfigFile::load(cnf)?;
    let def = ConfigFile::default_file().to_string_lossy().to_string();

    if opts.database.is_none() && cfg.database.is_none() {
        eprintln!(
            "Error: You must specify a database, either CLI or in {}",
            def
        );
        return Err(Status::ErrNoDatabase(def).into());
    }

    let name = opts.database.clone().unwrap();

    info!("Connecting to {}", name);
    let dbh = Connection::open_with_flags(
        name.as_str(),
        duckdb::Config::default()
            .allow_unsigned_extensions()?
            .enable_autoload_extension(true)?,
    )?;

    println!("Load extensions.");
    load_extensions(&dbh)?;

    let ctx = Context {
        config: HashMap::from([("database".to_string(), name.clone())]).into(),
        dbh: dbh.into(),
    };
    Ok(ctx)
}

/// Finish everything.
///
pub fn finish_runtime() -> Result<()> {
    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}

/// We need these extensions all the time.
///
#[tracing::instrument(skip(dbh))]
pub fn load_extensions(dbh: &Connection) -> Result<()> {
    // Load our extensions
    //
    dbh.execute("LOAD spatial", [])?;
    Ok(())
}
