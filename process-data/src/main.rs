//! Utility implement different processing tasks over our locally stored data.
//!

use clap::{crate_authors, crate_version, Parser};
use duckdb::Config;
use eyre::Result;
use std::env::Args;
use tracing::{info, trace};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use crate::cli::{Opts, SubCommand};

mod cli;
mod tasks;

use tasks::distance_calculation;

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

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

    info!("Connecting to {}", opts.database);
    let dbh = duckdb::Connection::open_with_flags(
        &opts.database,
        Config::default()
            .allow_unsigned_extensions()?
            .enable_autoload_extension(true)?,
    )?;

    match opts.subcmd {
        SubCommand::Distances => {
            let _ = distance_calculation(&dbh)?;
        }
        SubCommand::Various => {
            todo!()
        }
        SubCommand::Version => {
            eprintln!("{} v{}", NAME, VERSION);
        }
    }
    // Finish
    //
    opentelemetry::global::shutdown_tracer_provider();
    trace!("Closing DB.");
    let _ = dbh.close();
    Ok(())
}
