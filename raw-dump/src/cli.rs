use std::path::PathBuf;

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

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
    /// Sub-commands
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

/// fetch
/// import
/// list
#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Fetch data from specified site
    Fetch(FetchOpts),
    /// Import into InfluxDB
    Import(ImportOpts),
    /// Display possible sources
    List,
}

#[derive(Debug, Parser)]
pub struct ImportOpts {
    /// Sub-commands
    #[clap(subcommand)]
    pub subcmd: ImportSubCommand,
}

/// import file
/// import site
#[derive(Debug, Parser)]
pub enum ImportSubCommand {
    ImportFileOpts,
    ImportSiteOpts,
}

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

#[derive(Debug, Parser)]
pub struct ImportFileOpts {
    /// Format must be specified if looking at a file.
    #[clap(short = 'F', long)]
    pub format: Option<String>,
    /// File name (json expected)
    pub file: PathBuf,
}

#[derive(Debug, Parser)]
pub struct ImportSiteOpts {
    /// Start the data at specified date (optional)
    #[clap(short = 'B', long)]
    pub begin: Option<String>,
    /// End date (optional)
    #[clap(short = 'E', long)]
    pub end: Option<String>,
    /// We want today only
    #[clap(long)]
    pub today: bool,
    /// site name
    pub site: String,
}
