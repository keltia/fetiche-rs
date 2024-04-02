//! This is the `export` command module
//!

use clap::Parser;
use strum::{EnumString, VariantNames};

pub use distances::*;
pub use drones::*;
pub use encounters::*;

mod distances;
mod drones;
mod encounters;

#[derive(Clone, Copy, Debug, EnumString, VariantNames, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum Format {
    /// Classic CSV.
    Csv,
    /// Parquet compressed format.
    Parquet,
    /// Text for stdout
    Text,
}

#[derive(Debug, Parser)]
pub struct ExportOpts {
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: Option<String>,
    #[clap(subcommand)]
    pub subcmd: ExportSubCommand,
}

#[derive(Debug, Parser)]
pub enum ExportSubCommand {
    /// Export the distance calculations
    Distances(ExpDistOpts),
    /// Export daily or weekly stats for drones
    Drones(ExpDroneOpts),
    /// Export daily XML files for encounters
    Encounters(ExpEncOpts),
}
