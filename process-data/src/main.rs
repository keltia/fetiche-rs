//! Utility implement different processing tasks over our locally stored data.
//!

use clap::{crate_authors, crate_version, CommandFactory, Parser};
use clap_complete::generate;
use eyre::Result;
use std::io;
use tracing::{info, trace};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

use crate::cli::{Opts, SubCommand};
use crate::cmds::{
    cleanup_environment, connect_db, export_drone_stats, export_results, home_calculation,
    planes_calculation, run_acute_cmd, setup_acute_environment, DistSubcommand, ExportSubCommand,
};

mod cli;
mod cmds;
mod helpers;

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

    // Set up various things, add macros, etc.
    //
    trace!("Execute commands.");
    match opts.subcmd {
        SubCommand::Distances(opts) => {
            let dbh = connect_db(&opts.database)?;

            match opts.subcmd {
                DistSubcommand::Home => {
                    println!("Add 2D and 3D distance between drones and operator.");
                    home_calculation(&dbh)?;
                }
                DistSubcommand::Planes(popts) => {
                    println!("Calculate 3D distance between drones and surrounding planes.");
                    planes_calculation(&dbh, popts)?;
                }
            }
            info!("Closing DB.");
            let _ = dbh.close();
        }
        SubCommand::Export(opts) => {
            let dbh = connect_db(&opts.database)?;

            match opts.subcmd {
                ExportSubCommand::Distances(opts) => {
                    println!("Exporting calculated distances.");

                    export_results(&dbh, opts)?;
                }
                ExportSubCommand::Drones(opts) => {
                    println!("Exporting drone data.");

                    export_drone_stats(&dbh, opts)?;
                }
            }
            info!("Closing DB.");
            let _ = dbh.close();
        }
        SubCommand::Setup => {
            println!("Setup ACUTE environment.");
            let dbh = connect_db(&opts.database.unwrap())?;

            setup_acute_environment(&dbh)?;
            info!("Closing DB.");
            let _ = dbh.close();
        }
        SubCommand::Cleanup => {
            println!("Remove ACUTE specific macros and stuff.");
            let dbh = connect_db(&opts.database.unwrap())?;

            cleanup_environment(&dbh)?;
            info!("Closing DB.");
            let _ = dbh.close();
        }
        SubCommand::Acute(opts) => {
            println!("ACUTE specific commands.");
            let dbh = connect_db(&opts.database)?;

            run_acute_cmd(&dbh, opts)?;
            info!("Closing DB.");
            let _ = dbh.close();
        }
        SubCommand::Completion(copts) => {
            let generator = copts.shell;

            let mut cmd = Opts::command();
            generate(generator, &mut cmd, "acutectl", &mut io::stdout());
        }
        SubCommand::Version => {
            println!("{} v{}", NAME, VERSION);
        }
    }
    // Finish
    //
    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
