use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

/// CLI options
#[derive(Parser, Debug)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Start the data at specified date (optional)
    #[clap(short = 'B', long)]
    pub begin: Option<String>,
    /// configuration file.
    #[clap(short = 'c', long)]
    pub config: Option<PathBuf>,
    /// Duration in seconds (negative = back in time) -- optional
    #[clap(short = 'D', long)]
    pub since: Option<i32>,
    /// End date (optional)
    #[clap(short = 'E', long)]
    pub end: Option<String>,
    /// Format must be specified if looking at a file.
    #[clap(short = 'F', long)]
    pub format: Option<String>,
    /// Keyword filter: e.g. "--keyword icao24:foobar" -- optional
    #[clap(short = 'K', long)]
    pub keyword: Option<String>,
    /// Output file.
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// quiet mode.
    #[clap(short = 'q', long = "quiet")]
    pub quiet: bool,
    /// Site to fetch data from
    #[clap(short = 'S', long)]
    pub site: Option<String>,
    /// We want today only
    #[clap(long)]
    pub today: bool,
    /// Verbose mode.
    #[clap(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Display utility full version.
    #[clap(short = 'V', long)]
    pub version: bool,
    /// Input file.
    pub input: Option<PathBuf>,
}

/// Check the presence and validity of some of the arguments
///
pub fn check_args(opts: &Opts) -> Result<()> {
    // Check arguments.
    //
    if opts.input.is_some() && opts.site.is_some() {
        return Err(anyhow!("Specify either a site or a filename, not both"));
    }

    if opts.input.is_none() && opts.site.is_none() {
        return Err(anyhow!("Specify at least a site or a filename"));
    }

    if opts.input.is_some() && opts.format.is_none() {
        return Err(anyhow!("Format must be specified for files"));
    }

    // Do we have options for filter

    if opts.today && (opts.begin.is_some() || opts.end.is_some()) {
        return Err(anyhow!("Can not specify --today and -B/-E"));
    }

    if (opts.begin.is_some() && opts.end.is_none()) || (opts.begin.is_none() && opts.end.is_some())
    {
        return Err(anyhow!("We need both -B/-E or none"));
    }

    Ok(())
}
