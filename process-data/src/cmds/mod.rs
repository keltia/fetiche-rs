//! This is the main driver module for all the different commands.
//!

pub use acute::*;
pub use distances::*;
pub use export::*;
pub use setup::*;

use thiserror::Error;
use tracing::info;

use crate::cli::{Opts, SubCommand};
use crate::config::Context;

mod acute;
mod distances;
mod export;
mod setup;

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

#[tracing::instrument(skip(ctx))]
pub fn handle_cmds(ctx: &Context, opts: &Opts) -> eyre::Result<()> {
    match &opts.subcmd {
        SubCommand::Distances(dopts) => match &dopts.subcmd {
            DistSubcommand::Home => {
                println!("Add 2D and 3D distance between drones and operator.");
                home_calculation(&ctx)?;
            }
            DistSubcommand::Planes(popts) => {
                println!("Calculate 3D distance between drones and surrounding planes.");
                planes_calculation(&ctx, popts)?;
            }
        },
        SubCommand::Export(eopts) => match &eopts.subcmd {
            ExportSubCommand::Distances(opts) => {
                println!("Exporting calculated distances.");

                export_results(&ctx, opts)?;
            }
            ExportSubCommand::Drones(opts) => {
                println!("Exporting drone data.");

                export_drone_stats(&ctx, opts)?;
            }
        },
        SubCommand::Setup(sopts) => {
            println!("Setup ACUTE environment.");
            setup_acute_environment(&ctx, &sopts)?;
        }
        SubCommand::Cleanup(copts) => {
            println!("Remove ACUTE specific macros.");
            cleanup_environment(&ctx, &copts)?;
        }
        SubCommand::Acute(aopts) => {
            println!("ACUTE specific commands.");
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
