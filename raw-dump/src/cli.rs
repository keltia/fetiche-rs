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

#[derive(Parser)]
pub enum SubCommand {
    /// list-db displays possible sources
    Fetch(FetchOpts),
    /// fetch data from specified site
    ListDb,
}

#[derive(Parser)]
pub struct FetchOpts {
    /// Optional site name
    pub site: String,
}
