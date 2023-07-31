//! Module describing all possible commands and sub-commands to the `fetiched` daemon
//!

use std::net::IpAddr;
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
    /// debug mode (no fork & detach).
    #[clap(short = 'D', long = "debug")]
    pub debug: bool,
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
/// - `config`
/// - `server`
/// - `status`
/// - `version`
///
#[derive(Debug, Parser)]
pub enum SubCommand {
    /// Display current config
    Config,
    /// Run as a daemon (mostly for Windows)
    Server(ServerOpts),
    /// Shutdown everything
    Shutdown(ShutdownOpts),
    /// Daemon status
    Status,
    /// List all package versions
    Version,
}

/// Options for `server`
///
#[derive(Debug, Parser)]
pub struct ServerOpts {
    /// Configuration file
    #[clap(short = 'C', long)]
    pub config: Option<String>,
    /// Do not detach
    #[clap(short = 'D', long)]
    pub debug: bool,
    /// API listening IP, default is 127.0.0.1/::1
    #[clap(short = 'L', long, default_value = "::1")]
    pub listen: IpAddr,
    /// API port, default is 1998
    #[clap(short = 'P', long, default_value = "1998")]
    pub port: u16,
}

/// Options for `shutdown`
///
#[derive(Debug, Parser)]
pub struct ShutdownOpts {
    /// Optional delay
    pub delay: Option<usize>,
}