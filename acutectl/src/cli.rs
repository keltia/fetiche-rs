//! Module describing all possible commands and sub-commands to the `acutectl` main driver
//!
//!We have these commands:
//!
//! - `completion`
//! - `fetch`
//! - `convert`
//! - `list`
//! - `stream`
//! - `version`
//!
//! `fetch` retrieve the raw data (whether it is CSV, JSON or something else is not important) and dumps it
//! into a file or `stdout`.  `stream` does the same but run for either a specified time or forever,
//! waiting for a signal.
//!
//! Depending on the datatype for each source during `import`, `acutectl` does different processes.
//! We have a common format for drone data:
//!
//! `version` display all modules' version.
//!
//! `completion` is here just to configure the various shells completion system.
//!
//! A `Site` is a `Fetchable` or `Streamable`object with the corresponding trait methods (`authenticate()`
//! & `fetch()`/`stream()`) from the `sources` crate.  File formats are from the `formats` crate.
//!

use std::io;
use std::path::PathBuf;

use clap::{
    crate_authors, crate_description, crate_name, crate_version, CommandFactory, Parser, ValueEnum,
};
use clap_complete::generate;
use clap_complete::shells::Shell;
use eyre::Result;
use tracing::{info, trace};

use fetiche_common::{list_locations, load_locations, Container, DateOpts};
use fetiche_engine::Engine;
use fetiche_formats::Format;

use crate::{convert_from_to, fetch_from_site, stream_from_site};

/// CLI options
#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// configuration file.
    #[clap(short = 'c', long)]
    pub config: Option<String>,
    /// debug mode.
    #[clap(short = 'D', long = "debug")]
    pub debug: bool,
    /// Output file.
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Enable telemetry with OTLP.
    #[clap(short = 'T', long)]
    pub use_telemetry: bool,
    /// Enable logging in hierarchical manner (aka tree)
    #[clap(short = 'L', long)]
    pub use_tree: bool,
    /// This parameter enable logging to a file in that location.
    #[clap(short = 'F', long)]
    pub use_file: Option<String>,
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
    /// Convert between formats
    Convert(ConvertOpts),
    /// Fetch data from specified site
    Fetch(FetchOpts),
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
    /// Our different date options
    #[clap(subcommand)]
    pub dates: Option<DateOpts>,
    /// Duration in seconds (negative = back in time) -- optional
    #[clap(short = 'D', long)]
    pub since: Option<i32>,
    /// Keyword filter: e.g. "--keyword icao24:foobar" -- optional
    #[clap(short = 'K', long)]
    pub keyword: Option<String>,

    // General options
    //
    /// Output file -- default is stdout
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Create a copy of the raw file before any conversion
    #[clap(long)]
    pub tee: Option<String>,
    /// Do we convert on streaming?
    #[clap(long, value_parser)]
    pub into: Option<Format>,
    /// Output format (if needed, like for parquet)
    #[clap(long, value_parser)]
    pub write: Option<Container>,
    /// Source name -- (see "list sources")
    pub site: String,
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
    /// List all commands in `Engine`
    Commands,
    /// Lists all supported write/container formats.
    Containers,
    /// List all formats in `formats`
    Formats,
    /// List all possible sites for antennas.
    Sites,
    /// List all sources from `sources.hcl`
    Sources,
    /// List all storage areas
    Storage,
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
    /// Create a copy of the raw file before any conversion
    #[clap(long)]
    pub tee: Option<String>,
    /// Do we convert on streaming?
    #[clap(long)]
    pub into: Option<String>,
    /// Do we want split output?
    #[clap(long)]
    pub split: Option<String>,
    /// Source name -- (see "list sources")
    pub site: String,
}

// -----

/// Options for the `convert` command, take a filename and format
///
#[derive(Debug, Parser)]
pub struct ConvertOpts {
    /// Input format
    #[clap(long)]
    pub from: Format,
    /// Output format
    #[clap(long)]
    pub into: Format,
    /// Input file
    pub infile: String,
    /// Output file
    pub outfile: String,
}

#[tracing::instrument(skip(engine))]
pub fn handle_subcmd(engine: &mut Engine, subcmd: &SubCommand) -> Result<()> {
    match subcmd {
        // Handle `fetch site`
        //
        SubCommand::Fetch(fopts) => {
            trace!("fetch");

            fetch_from_site(engine, fopts)?;
        }

        // Handle `stream site`
        //
        SubCommand::Stream(sopts) => {
            trace!("stream");

            stream_from_site(engine, sopts)?;
        }

        // Handle `convert from to`
        //
        SubCommand::Convert(copts) => {
            trace!("convert");

            convert_from_to(engine, copts)?;
        }

        // Standalone completion generation
        //
        // NOTE: you can generate UNIX shells completion on Windows and vice-versa.  Not worth
        //       trying to limit depending on the OS.
        //
        SubCommand::Completion(copts) => {
            let generator = copts.shell;

            let mut cmd = Opts::command();
            generate(generator, &mut cmd, "acutectl", &mut io::stdout());
        }

        // Standalone `list` command
        //
        SubCommand::List(lopts) => match lopts.cmd {
            ListSubCommand::Commands => {
                info!("Listing all commands:");

                let str = engine.list_commands()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Containers => {
                info!("Listing all container formats:");

                let str = engine.list_containers()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Sources => {
                info!("Listing all sources:");

                let str = engine.list_sources()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Sites => {
                info!("Listing all sites:");

                let list = load_locations(None)?;
                let str = list_locations(&list, 70)?;
                eprintln!("{}", str);
            }
            ListSubCommand::Formats => {
                info!("Listing all formats:");

                let str = engine.list_formats()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Tokens => {
                info!("Listing all tokens:");

                let str = engine.list_tokens()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Storage => {
                info!("Listing all storage areas:");

                let str = engine.list_storage()?;
                eprintln!("{}", str);
            }
        },

        // Standalone `version` command
        //
        SubCommand::Version => {
            eprintln!("Modules: \t{}", engine.version());
        }
    }
    Ok(())
}
