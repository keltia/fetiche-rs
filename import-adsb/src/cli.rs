use std::path::PathBuf;

use anyhow::{anyhow, Result};
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
    /// DB connection file
    #[clap(short = 'd', long)]
    pub dbfile: Option<PathBuf>,
    /// debug mode.
    #[clap(short = 'D', long = "debug")]
    pub debug: bool,
    /// Format must be specified if looking at a file.
    #[clap(short = 'F', long)]
    pub format: Option<String>,
    /// Site to fetch data from
    #[clap(short = 'S', long)]
    pub site: Option<String>,
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
    CreateDb(CreateOpts),
    Import(ImportOpts),
}

#[derive(Parser)]
pub struct CreateOpts {}

#[derive(Parser)]
pub struct ImportOpts {}
