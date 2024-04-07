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
mod batch;
mod distances;
mod export;
mod setup;
mod stats;

/// One degree in *kilometers*
const ONE_DEG: f64 = 40_000. / 360.;

#[derive(Debug, Error)]
pub enum Status {
    #[error("Invalid site name {0}")]
    UnknownSite(String),
    #[error("No database specified anywhere (config: {0}")]
    NoDatabase(String),
    #[error("No datalake specified in {0}")]
    NoDatalake(String),
}

// -----

#[tracing::instrument(skip(ctx))]
pub async fn handle_cmds(ctx: &Context, opts: &Opts) -> eyre::Result<()> {
    match &opts.subcmd {
        SubCommand::Distances(dopts) => match &dopts.subcmd {
            DistSubcommand::Planes(popts) => {
                println!("Calculate 3D distance between drones and surrounding planes.\n");

                let stats = planes_calculation(ctx, popts)?;
                println!("Stats:\n{:?}", stats);
            }
        },
        SubCommand::Export(eopts) => match &eopts.subcmd {
            ExportSubCommand::Distances(opts) => {
                println!("Exporting calculated distances.\n");

                export_results(ctx, opts)?;
            }
            ExportSubCommand::Drones(opts) => {
                println!("Exporting drone data.\n");

                export_drone_stats(ctx, opts)?;
            }
            ExportSubCommand::Encounters(opts) => unimplemented!()
        },
        SubCommand::Setup(sopts) => {
            println!("Setup ACUTE environment in {}.\n", ctx.config["datalake"]);
            setup_acute_environment(ctx, sopts).await?;
        }
        SubCommand::Cleanup(copts) => {
            println!("Remove ACUTE environement in {}.\n", ctx.config["datalake"]);
            cleanup_environment(ctx, copts).await?;
        }
        SubCommand::Acute(aopts) => {
            println!("ACUTE specific commands.\n");
            run_acute_cmd(ctx, aopts)?;
        }
        // These are done already.
        //
        _ => (),
    }

    info!("Closing DB.");
    let _ = ctx.finish();

    Ok(())
}
