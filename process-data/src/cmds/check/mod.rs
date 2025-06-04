//! `check` command module
//!

use clap::Parser;

use fetiche_common::DateOpts;

/// Options for the check command
#[derive(Debug, Parser)]
pub struct CheckOpts {
    #[clap(subcommand)]
    pub cmd: CheckSubCommand,
}

/// Subcommands available for the check command
#[derive(Debug, Parser)]
pub enum CheckSubCommand {
    /// Check for completed runs.
    #[clap(visible_alias = "c", visible_alias = "compl")]
    Completed(ComplOpts),
    /// Check for missing data
    #[clap(visible_alias = "m", visible_alias = "miss")]
    Missing(MissingOpts),
}

/// Options for checking missing data
#[derive(Debug, Parser)]
pub struct MissingOpts {
    #[clap(subcommand)]
    pub cmd: MissingSubCommand,
}

#[derive(Debug, Parser)]
pub enum MissingSubCommand {
    /// Missing ADS-B data for all days & sites.
    #[clap(visible_alias = "a")]
    Adsb(MAdsbOpts),
    /// Missing drone data for all days & sites.
    #[clap(visible_alias = "d")]
    Drones(MDronesOpts),
}

/// Options for checking completed runs
#[derive(Debug, Parser)]
pub struct ComplOpts {
    /// Check on a given day.
    #[clap(subcommand)]
    day: Option<DateOpts>,
    /// Check for a given site.
    site: Option<String>,
}

/// Options for checking missing ADS-B data
#[derive(Debug, Parser)]
pub struct MAdsbOpts {
    /// Check on a given day.
    #[clap(subcommand)]
    day: Option<DateOpts>,
    /// Check for a given site.
    site: Option<String>,
}

/// Options for checking missing drone data
#[derive(Debug, Parser)]
pub struct MDronesOpts {
    /// Check on a given day.
    #[clap(subcommand)]
    day: Option<DateOpts>,
    /// Check for a given site.
    site: Option<String>,
}
