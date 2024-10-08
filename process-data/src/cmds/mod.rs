//! This is the main driver module for all the different commands.
//!

use klickhouse::Client;
use std::fmt::Debug;
use tracing::info;

pub use acute::*;
pub use distances::*;
pub use export::*;
//pub use import::*;
pub use setup::*;
pub use site::*;
pub use stats::*;

use crate::cli::{Opts, SubCommand};
use crate::config::Context;

mod acute;
mod distances;
mod export;
//mod import;
mod setup;
mod site;
mod stats;

/// One degree in *kilometers*
const ONE_DEG: f64 = 40_000. / 360.;

/// This trait define an object that can be calculated
///
pub trait Calculate: Debug {
    async fn run(&mut self, dbh: &Client) -> eyre::Result<Stats>;
}

// -----

#[tracing::instrument(skip(ctx))]
pub async fn handle_cmds(ctx: &Context, opts: &Opts) -> eyre::Result<()> {
    match &opts.subcmd {
        SubCommand::Distances(dopts) => match &dopts.subcmd {
            DistSubcommand::Planes(popts) => {
                eprintln!("Calculate 3D distance between drones and surrounding planes.\n");

                let stats = planes_calculation(ctx, popts).await?;
                eprintln!("Stats:\n{:?}", stats);
            }
        },
        SubCommand::Export(eopts) => match &eopts.subcmd {
            ExportSubCommand::Distances(opts) => {
                eprintln!("Exporting calculated distances.\n");

                export_results(ctx, opts).await?;
            }
            ExportSubCommand::Drones(opts) => {
                eprintln!("Exporting drone data.\n");

                export_drone_stats(ctx, opts).await?;
            }
        },
        SubCommand::Setup(sopts) => {
            eprintln!("Setup ACUTE environment in {}.\n", ctx.config["datalake"]);
            setup_acute_environment(ctx, sopts).await?;
        }
        SubCommand::Cleanup(copts) => {
            eprintln!("Remove ACUTE environement in {}.\n", ctx.config["datalake"]);
            cleanup_environment(ctx, copts).await?;
        }
        SubCommand::Acute(aopts) => {
            eprintln!("ACUTE specific commands.\n");
            run_acute_cmd(ctx, aopts).await?;
        }
        // These are done already.
        //
        _ => (),
    }

    info!("Closing DB.");
    let _ = ctx.finish();

    Ok(())
}
