//! Utility implement different processing tasks over our locally stored data.
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

    banner()?;

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
fn banner() -> Result<()> {
    let ver = format!("{} v{}+clickhouse", NAME, VERSION);
    eprintln!(
        r##"
{ver} by {AUTHORS}
{}
"##,
        crate_description!()
    );
    Ok(())
}
