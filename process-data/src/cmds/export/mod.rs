//! This is the `export` command module
//!
//! NOTE: The HTTP API does not really support this.  You need to use `clickhouse-client` or
//! `clickhouse client` to export such query results.  Therefore, this is for reference.
//!
//! FIXME: Using the `klickhouse` API might fix this as it uses the client access (port 9000/tcp).
//!

use clap::Parser;
use strum::{EnumString, VariantNames};

pub use distances::*;
pub use drones::*;

mod distances;
mod drones;

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
}
