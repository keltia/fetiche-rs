//! This is the main driver module for all the different commands.
//!

use thiserror::Error;
use tracing::info;

pub use acute::*;
pub use distances::*;
pub use export::*;
pub use setup::*;
pub use stats::*;

use crate::cli::{Opts, SubCommand};
use crate::config::Context;

mod acute;
mod distances;
mod export;
mod setup;
mod stats;

/// One degree in *kilometers*
const ONE_DEG: f64 = 40_000. / 360.;

#[derive(Debug, Error)]
pub enum Status {
    #[error("No planes were found around site {0} at this date")]
    NoPlanesFound(String),
    #[error("No drones in the {0} area")]
    NoDronesFound(String),
    #[error("No encounters found in the {0} area")]
    NoEncounters(String),
    #[error("Invalid site name {0}")]
    ErrUnknownSite(String),
    #[error("No database specified anywhere (config: {0}")]
    ErrNoDatabase(String),
    #[error("No datalake specified in {0}")]
    ErrNoDatalake(String),
}

// -----

#[tracing::instrument(skip(ctx))]
pub fn handle_cmds(ctx: &Context, opts: &Opts) -> eyre::Result<()> {
    match &opts.subcmd {
        SubCommand::Distances(dopts) => match &dopts.subcmd {
            DistSubcommand::Home => {
                println!("Add 2D and 3D distance between drones and operator.\n");

                let stats = home_calculation(&ctx)?;
                println!("Stats:\n{:?}", stats);
            }
            DistSubcommand::Planes(popts) => {
                println!("Calculate 3D distance between drones and surrounding planes.\n");

                let stats = planes_calculation(&ctx, popts)?;
                println!("Stats:\n{:?}", stats);
            }
        },
        SubCommand::Export(eopts) => match &eopts.subcmd {
            ExportSubCommand::Distances(opts) => {
                println!("Exporting calculated distances.\n");

                export_results(&ctx, opts)?;
            }
            ExportSubCommand::Drones(opts) => {
                println!("Exporting drone data.\n");

                export_drone_stats(&ctx, opts)?;
            }
        },
        SubCommand::Setup(sopts) => {
            println!("Setup ACUTE environment.\n");
            setup_acute_environment(&ctx, &sopts)?;
        }
        SubCommand::Cleanup(copts) => {
            println!("Remove ACUTE specific macros.\n");
            cleanup_environment(&ctx, &copts)?;
        }
        SubCommand::Acute(aopts) => {
            println!("ACUTE specific commands.\n");
            run_acute_cmd(&ctx, &aopts)?;
        }
        // These are done already.
        //
        _ => (),
    }

    info!("Closing DB.");
    let _ = ctx.finish();

    Ok(())
}
