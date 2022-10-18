use std::path::PathBuf;

use chrono::{DateTime, Utc};
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

/// CLI options
#[derive(Parser, Debug)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Start the data at specified date (optional)
    #[clap(short = 'B', long)]
    pub begin: Option<DateTime<Utc>>,
    /// configuration file.
    #[clap(short = 'c', long)]
    pub config: Option<PathBuf>,
    /// debug mode.
    #[clap(short = 'D', long = "debug")]
    pub debug: bool,
    /// End date (optional)
    #[clap(short = 'E', long)]
    pub end: Option<DateTime<Utc>>,
    /// Output file.
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Optional password.
    #[clap(short = 'P', long)]
    pub password: Option<String>,
    /// Site to fetch data from
    #[clap(short = 'S', long)]
    pub site: Option<String>,
    /// We want today only
    #[clap(long)]
    pub today: bool,
    /// Optional username for the server API.
    #[clap(short = 'U', long)]
    pub username: Option<String>,
    /// Verbose mode.
    #[clap(short = 'v', long)]
    pub verbose: Option<usize>,
    /// Display utility full version.
    #[clap(short = 'V', long)]
    pub version: bool,
    /// Input file.
    pub input: Option<PathBuf>,
}
