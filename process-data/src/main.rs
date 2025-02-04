//! # process-data
//!
//! `process-data` is a command-line utility for performing various data processing tasks 
//! on locally stored data. It provides a flexible and extensible framework to handle diverse 
//! operations using modern Rust features and libraries.
//!
//! ## Features
//!
//! - Modular command structure with subcommands for different tasks
//! - Auto-completion support for various shells
//! - Integration with Clickhouse for data handling
//! - Asynchronous processing powered by Tokio
//! - Comprehensive logging and error handling
//! - Configurable runtime settings
//!
//! ## Usage
//!
//! Run `process-data --help` to display usage information and available commands.
//!

use std::io;

use clap::{crate_authors, crate_description, crate_version, CommandFactory, Parser};
use clap_complete::generate;
use eyre::Result;
use tracing::trace;

use crate::cli::{Opts, SubCommand};
use crate::cmds::handle_cmds;
use crate::config::{finish_runtime, init_runtime};

mod cli;
mod cmds;
mod config;
mod error;

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

/// Use reasonable defaults for tokio threads & workers.
///
#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    // Initialise our context including logging.
    //
    let ctx = init_runtime(&opts).await?;

    eprintln!("{}", banner()?);

    trace!("Execute commands.");
    match &opts.subcmd {
        SubCommand::Completion(copts) => {
            let generator = copts.shell;
            eprintln!("Generating completion file for {}", generator);

            let mut cmd = Opts::command();
            generate(generator, &mut cmd, NAME, &mut io::stdout());
        }
        SubCommand::Version => {
            eprintln!("{} v{}+clickhouse", NAME, VERSION);
        }
        _ => handle_cmds(&ctx, &opts).await?,
    }

    // Finish
    //
    finish_runtime(&ctx)
}

/// Display banner
///
fn banner() -> Result<String> {
    let ver = format!("{} v{}+clickhouse", NAME, VERSION);
    Ok(format!(
        r##"
{ver} by {AUTHORS}
{}
"##,
        crate_description!()
    ))
}
