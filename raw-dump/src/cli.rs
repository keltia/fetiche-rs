//! Module describing all possible commands and sub-commands to the `acutectl` main driver
//!
//!We have three main commands:
//!
//! - `adsb`
//! - `drone`
//! - `list`
//!
//! Both `adsb` and `drone` accept the sub-commands `fetch` & `import`, the main difference is
//! what is done with the data itself.  Every drone source is converted into state vectors of `DronePoint`
//! with a timestamp suitable for import into a time-series DB.  ADS-B data will use a different
//! format with more fields related to planes.
//!
//! `fetch` retrieve the raw data (whether it is CSV, JSON or something else is not important) and dumps it
//! into a file or `stdout`.
//!
//! `import` convert data into a data format suitable for importing into a time-series database
//! ([InfluxDB] at the moment).
//!
//! A `Site` is a `Fetchable` object with the corresponding trait methods (`authenticate()` & `fetch()`)
//! from the `sources` crate.  File formats are from the `format-specs` crate.
//!
//! [InfluxDB]: https://www.influxdata.com/
//!

use std::path::PathBuf;

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser, Subcommand};

/// CLI options
#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// configuration file.
    #[clap(short = 'c', long)]
    pub config: Option<PathBuf>,
    /// debug mode.
    #[clap(short = 'D', long = "debug")]
    pub debug: bool,
    /// Output file.
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Verbose mode.
    #[clap(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Display utility full version.
    #[clap(short = 'V', long)]
    pub version: bool,
    /// Sub-commands (see below).
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

// ------

/// All sub-commands:
///
/// `adsb (fetch \ import)`
/// `drone (fetch | import)`
/// `list`
///
#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Handle ADS-B data
    Adsb(AdsbOpts),
    /// Handle drone data
    Drone(DroneOpts),
    /// Display possible sources
    List,
}

// ------

/// All `drone`subcommands:
///
/// `drone fetch [-B date] [-E date] [--today] [-o FILE] site`
/// `drone import (file|site)`
///
#[derive(Debug, Parser)]
pub struct DroneOpts {
    #[clap(subcommand)]
    pub subcmd: DroneSubCommand,
}

#[derive(Debug, Parser)]
pub enum DroneSubCommand {
    /// Fetch data from specified site
    Fetch(FetchOpts),
    /// Import into InfluxDB
    Import(ImportOpts),
}

// ------

/// All `adsb`subcommands:
///
/// `adsb fetch [-B date] [-E date] [--today] [-o FILE] site`
/// `adsb import (file|site)`
///
#[derive(Debug, Parser)]
pub struct AdsbOpts {
    #[clap(subcommand)]
    pub subcmd: AdsbSubCommand,
}

#[derive(Debug, Parser)]
pub enum AdsbSubCommand {
    /// Fetch data from specified site
    Fetch(FetchOpts),
    /// Import into InfluxDB
    Import(ImportOpts),
}

// ------

/// This contain only the `import` sub-commands.
///
#[derive(Debug, Parser)]
pub struct ImportOpts {
    /// Sub-commands
    #[clap(subcommand)]
    pub subcmd: ImportSubCommand,
}

// ------

/// All `import` sub-commands:
///
/// `import file {-F format] path`
/// `import site [-B date] [-E date] [--today] site`
///
#[derive(Debug, Parser)]
pub enum ImportSubCommand {
    /// Import from file
    ImportFile(ImportFileOpts),
    /// Import from site, using options as fetch
    ImportSite(FetchOpts),
}

#[derive(Debug, Parser)]
pub struct ImportFileOpts {
    /// Format must be specified if looking at a file.
    #[clap(short = 'F', long)]
    pub format: Option<String>,
    /// File name (json expected)
    pub file: PathBuf,
}

// ------

/// Shared options for fetching data with basic filtering and an optional output file.
///
#[derive(Debug, Parser)]
pub struct FetchOpts {
    /// Start the data at specified date (optional)
    #[clap(short = 'B', long)]
    pub begin: Option<String>,
    /// End date (optional)
    #[clap(short = 'E', long)]
    pub end: Option<String>,
    /// Output file.
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// We want today only
    #[clap(long)]
    pub today: bool,
    /// site name
    pub site: String,
}
