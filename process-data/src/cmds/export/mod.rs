//! This is the `export` command module
//!

mod distances;

use chrono::{DateTime, Datelike, TimeZone, Utc};
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
    pub subcmd: ExportSubcommand,
}

#[derive(Debug, Parser)]
pub enum ExportSubcommand {
    /// Export the distance calculations
    Distances(ExpDistOpts),
    /// Export daily or weekly stats for drones
    Drones(ExpDroneOpts),
}

#[derive(Debug, Parser)]
pub struct ExpDistOpts {
    /// Export results for this site
    pub name: String,
    /// Day to export
    pub date: String,
    /// Output format
    #[clap(short = 'F', long, default_value = "csv")]
    pub format: Format,
    /// Output file
    #[clap(short = 'o', long)]
    pub output: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ExpDroneOpts {
    /// Specific day.
    pub day: Option<DateTime<Utc>>,
    /// Specific week number
    pub week: Option<usize>,
    /// Output format
    #[clap(short = 'F', long, default_value = "csv")]
    pub format: Format,
    /// Output file
    #[clap(short = 'o', long)]
    pub output: Option<String>,
}
