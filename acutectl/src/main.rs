//! Main driver for fetching various data from sites and API.
//!
//! Usage:
//!
//! ```text
//! CLI utility to fetch data.
//
// Usage: acutectl.exe [OPTIONS] <COMMAND>
//
// Commands:
//   completion  Generate Completion stuff
//   convert     Convert between formats
//   fetch       Fetch data from specified site
//   list        List information about formats and sources
//   stream      Stream from a source
//   version     List all package versions
//   help        Print this message or the help of the given subcommand(s)
//
// Options:
//   -c, --config <CONFIG>  configuration file
//   -D, --debug            debug mode
//   -o, --output <OUTPUT>  Output file
//   -v, --verbose...       Verbose mode
//   -h, --help             Print help
//! ```

mod init;

use clap::{crate_authors, crate_description, crate_version, Parser};
use eyre::Result;
use tracing::trace;

use crate::init::init_runtime;

use acutectl::{handle_subcmd, Config, Engine, Opts};

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

fn main() -> Result<()> {
    let opts = Opts::parse();
    let cfn = opts.config.clone();

    // Initialise tracing.
    //
    init_runtime(NAME)?;

    // Config only has the credentials for every source now.
    //
    let cfg = Config::load(cfn)?;

    // Banner
    //
    banner()?;

    trace!("Engine starting.");
    // Instantiate Engine
    //
    let mut engine = Engine::new();

    // Load auth data
    //
    engine.auth(cfg.site);

    trace!("Engine initialised and running.");

    let subcmd = &opts.subcmd;
    handle_subcmd(&mut engine, subcmd)
}

/// Return our version number
///
#[inline]
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
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
