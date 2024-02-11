//! `export drones`  sub-module.
//!

use chrono::{DateTime, Utc};
use clap::Parser;
use duckdb::Connection;

use crate::cmds::Format;

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

#[tracing::instrument(skip(dbh))]
pub fn export_drone_stats(dnh: &Connection, opts: ExpDroneOpts) -> eyre::Result<()> {
    Ok(())
}
