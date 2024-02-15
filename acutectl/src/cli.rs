//! Module describing all possible commands and sub-commands to the `acutectl` main driver
//!
//!We have these commands:
//!
//! - `completion`
//! - `fetch`
//! - `convert`
//! - `import`
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
//! `import` convert data into a data format suitable for importing into a database
//! ([InfluxDB] at the moment).
//!
//! `completion` is here just to configure the various shells completion system.
//!
//! A `Site` is a `Fetchable` or `Streamable`object with the corresponding trait methods (`authenticate()`
//! & `fetch()`/`stream()`) from the `sources` crate.  File formats are from the `formats` crate.
//!
//! [InfluxDB]: https://www.influxdata.com/
//!

use std::path::PathBuf;
use std::str::FromStr;
use std::{fs, io};

use clap::{
    crate_authors, crate_description, crate_name, crate_version, CommandFactory, Parser, ValueEnum,
};
use clap_complete::generate;
use clap_complete::shells::Shell;
use eyre::{eyre, Result};
use tracing::{info, trace};

use fetiche_formats::{Format, Write};
use fetiche_sources::{Flow, Site};

use crate::{convert_from_to, fetch_from_site, stream_from_site, Engine, FileInput};

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
    pub output: Option<String>,
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
    pub write: Option<Write>,
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
    /// Database to send the data into
    #[clap(short = 'd', long)]
    pub db: String,
    /// URL to connect to (see `config.hcl`)
    pub url: Option<String>,
    /// Format must be specified if looking at a file.
    #[clap(short = 'F', long, default_value = "csv", value_parser)]
    pub format: Option<FileInput>,
    /// File name (json/csv/parquet expected)
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
    /// List all commands in `Engine`
    Commands,
    /// Lists all supported write/container formats.
    Containers,
    /// List all formats in `formats`
    Formats,
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

        // Handle `import site`  and `import file`
        // FIXME:
        //
        SubCommand::Import(opts) => {
            trace!("import");

            match &opts.subcmd {
                ImportSubCommand::ImportSite(fopts) => {
                    trace!("drone import site");

                    let srcs = &engine.sources();
                    let site = match Site::load(&fopts.site, srcs)? {
                        Flow::Fetchable(s) => s,
                        _ => return Err(eyre!("this site is not fetchable")),
                    };
                    let fmt = site.format();

                    // FIXME
                    let data: Vec<u8> = vec![];

                    fetch_from_site(engine, fopts)?;

                    //import_data(&cfg, &data, fmt)?;
                }
                ImportSubCommand::ImportFile(if_opts) => {
                    trace!("db import file");

                    let db = &if_opts.db;
                    let data = fs::read_to_string(&if_opts.file)?;
                    let fmt = Format::from_str(&if_opts.format.unwrap().to_string())?;

                    //import_data(&srcs, &data, fmt)?;
                }
            }
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
    opentelemetry::global::shutdown_tracer_provider();

    Ok(())
}
