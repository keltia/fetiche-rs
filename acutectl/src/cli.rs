//! Module describing all possible commands and sub-commands to the `acutectl` main driver
//!
//!We have three main commands:
//!
//! - `fetch`
//! - `import`
//! - `list`
//!
//! `fetch` retrieve the raw data (whether it is CSV, JSON or something else is not important) and dumps it
//! into a file or `stdout`.
//!
//! Depending on the datatype for each source during `import`, `acutectl` does different processes.
//! We have a common format for drone data:
//!
//! Every drone source is converted into state vectors of `DronePoint` with a timestamp suitable for
//! import into a time-series DB.  ADS-B data will use a different format with more fields related
//! to planes.
//!
//! `import` convert data into a data format suitable for importing into a database
//! ([InfluxDB] at the moment).
//!
//! `completion` is here just to configure the various shells completion system.
//!
//! A `Site` is a `Fetchable` object with the corresponding trait methods (`authenticate()` & `fetch()`)
//! from the `sources` crate.  File formats are from the `formats` crate.
//!
//! [InfluxDB]: https://www.influxdata.com/
//!

use std::path::PathBuf;

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser, ValueEnum};
use clap_complete::shells::Shell;

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
    /// Sub-commands (see below).
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

// ------

/// All sub-commands:
///
/// `completion SHELL`
/// `fetch [-B date] [-E date] [--today] [-o FILE] site`
/// `import (file|site) OPTS`
/// `list`
///
#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Generate Completion stuff
    Completion(ComplOpts),
    /// Fetch data from specified site
    Fetch(FetchOpts),
    /// Import into InfluxDB (WIP)
    Import(ImportOpts),
    /// List information about formats and sources
    List(ListOpts),
    /// Stream from a source
    Stream(StreamOpts),
    /// List all package versions
    Version,
}

// ------

/// Options for fetching data with basic filtering and an optional output file.
///
#[derive(Debug, Parser)]
pub struct FetchOpts {
    /// We want today only
    #[clap(long)]
    pub today: bool,
    /// Start date - YYYY-MM-DD HH:MM:SS -- optional
    #[clap(short = 'B', long)]
    pub begin: Option<String>,
    /// End date - YYYY-MM-DD HH:MM:SS -- optional
    #[clap(short = 'E', long)]
    pub end: Option<String>,
    /// Duration in seconds (negative = back in time) -- optional
    #[clap(short = 'D', long)]
    pub since: Option<i32>,
    /// Keyword filter: e.g. "--keyword icao24:foobar" -- optional
    #[clap(short = 'K', long)]
    pub keyword: Option<String>,
    /// Output file -- default is stdout
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Source name -- (see "list sources")
    pub site: String,
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

/// Options to generate completion files at runtime
///
#[derive(Debug, Parser)]
pub struct ComplOpts {
    #[clap(value_parser)]
    pub shell: Shell,
}

// ------

/// All  list` sub-commands:
///
/// `list formats`
/// `list sources`
///
#[derive(Debug, Parser)]
pub struct ListOpts {
    #[clap(value_parser)]
    pub cmd: ListSubCommand,
}

/// These are the sub-commands for `list
///
#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, ValueEnum)]
pub enum ListSubCommand {
    /// List all formats in `formats`
    Formats,
    /// List all sources from `sources.hcl`
    Sources,
    /// List all currently stored tokens
    Tokens,
}

// -----

/// Options for fetching data with basic filtering and an optional output file.
///
#[derive(Debug, Parser)]
pub struct StreamOpts {
    // ASD
    //
    /// We want today only
    #[clap(long)]
    pub today: bool,
    /// Start date - YYYY-MM-DD HH:MM:SS -- optional
    #[clap(short = 'B', long)]
    pub begin: Option<String>,
    /// End date - YYYY-MM-DD HH:MM:SS -- optional
    #[clap(short = 'E', long)]
    pub end: Option<String>,

    // Opensky
    //
    /// Start the stream at EPOCH + `start`
    #[clap(short = 'S', long)]
    pub start: Option<i64>,
    /// Duration in seconds (negative = back in time) -- default to 0 (do not stop)
    #[clap(short = 'D', long, default_value = "0")]
    pub duration: u32,
    /// Keyword filter: e.g. "--keyword icao24:foobar" -- optional
    #[clap(short = 'K', long)]
    pub keyword: Option<String>,
    /// Insert a slight delay between calls in ms, default is 1000
    #[clap(long, default_value = "1000")]
    pub delay: u32,

    // General options
    //
    /// Output file -- default is stdout
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Source name -- (see "list sources")
    pub site: String,
}
