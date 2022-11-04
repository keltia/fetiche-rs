use std::path::PathBuf;

use chrono::NaiveDateTime;
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

/// CLI options
#[derive(Parser, Debug)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Start the data at specified date (optional)
    #[clap(short = 'B', long)]
    pub begin: Option<NaiveDateTime>,
    /// configuration file.
    #[clap(short = 'c', long)]
    pub config: Option<PathBuf>,
    /// debug mode.
    #[clap(short = 'D', long = "debug")]
    pub debug: bool,
    /// End date (optional)
    #[clap(short = 'E', long)]
    pub end: Option<NaiveDateTime>,
    /// Format must be specified if looking at a file.
    #[clap(short = 'F', long)]
    pub format: Option<String>,
    /// Output file.
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Site to fetch data from
    #[clap(short = 'S', long)]
    pub site: Option<String>,
    /// We want today only
    #[clap(long)]
    pub today: bool,
    /// Verbose mode.
    #[clap(short = 'v', long)]
    pub verbose: Option<usize>,
    /// Display utility full version.
    #[clap(short = 'V', long)]
    pub version: bool,
    /// Input file.
    pub input: Option<PathBuf>,
}
