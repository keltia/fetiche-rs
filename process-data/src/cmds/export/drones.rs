//! `export drones`  sub-module.
//!

use chrono::{DateTime, Utc};
use clap::Parser;

use crate::cmds::Format;
use crate::config::Context;

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

#[tracing::instrument(skip(_ctx))]
pub fn export_drone_stats(_ctx: &Context, _opts: &ExpDroneOpts) -> eyre::Result<()> {
    Ok(())
}
