use std::path::PathBuf;

use clap::{crate_authors, crate_name, crate_version, Parser};

pub const ABOUT: &str = "Fetch the latest SAC codes data from ECTL.\n\
Source: https://www.eurocontrol.int/asterix/";

/// CLI options
#[derive(Parser, Debug)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name ! (), about = ABOUT)]
#[clap(version = crate_version ! (), author = crate_authors ! ())]
pub struct Opts {
    /// CSV
    #[clap(short = 'C', long)]
    pub csv: bool,
    /// JSON
    #[clap(short = 'J', long)]
    pub json: bool,
    /// Output file.
    #[clap(short = 'o', long)]
    pub output: Option<PathBuf>,
    /// Verbose mode.
    #[clap(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Display utility full version.
    #[clap(short = 'V', long)]
    pub version: bool,
}
