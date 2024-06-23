//! Utility implement different processing tasks over our locally stored data.
//!

use clap::{crate_authors, crate_description, crate_version, CommandFactory, Parser};
use clap_complete::generate;
use eyre::Result;
use std::io;
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

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    // Initialise our context including logging.
    //
    let ctx = init_runtime(&opts)?;

    banner()?;

    trace!("Execute commands.");
    match &opts.subcmd {
        SubCommand::Completion(copts) => {
            let generator = copts.shell;

            let mut cmd = Opts::command();
            generate(generator, &mut cmd, "acutectl", &mut io::stdout());
        }
        SubCommand::Version => {
            println!("{} v{}", NAME, VERSION);
        }
        _ => handle_cmds(&ctx, &opts).await?,
    }

    // Finish
    //
    finish_runtime()
}

/// Display banner
///
fn banner() -> Result<()> {
    Ok(eprintln!(
        r##"
{}/{} by {}
{}
"##,
        NAME,
        VERSION,
        AUTHORS,
        crate_description!()
    ))
}
