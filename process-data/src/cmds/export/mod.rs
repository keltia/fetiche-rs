//! This is the `export` command module
//!

mod distances;
mod drones;

pub use distances::*;
pub use drones::*;

use clap::Parser;
use strum::{EnumString, VariantNames};

#[derive(Clone, Copy, Debug, EnumString, VariantNames, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum Format {
    /// Classic CSV.
    Csv,
    /// Parquet compressed format.
    Parquet,
}

#[derive(Debug, Parser)]
pub struct ExportOpts {
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: String,
    #[clap(subcommand)]
    pub subcmd: ExportSubCommand,
}

#[derive(Debug, Parser)]
pub enum ExportSubCommand {
    /// Export the distance calculations
    Distances(ExpDistOpts),
    /// Export daily or weekly stats for drones
    Drones(ExpDroneOpts),
}
