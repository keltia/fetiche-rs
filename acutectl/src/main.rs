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

use clap::{crate_authors, crate_description, crate_version, Parser};
use eyre::{eyre, Result};
use tracing::trace;

use acutectl::{handle_subcmd, Config, Opts, Status, CVERSION};
use fetiche_common::{close_logging, init_logging, ConfigFile, Versioned};
use fetiche_engine::Engine;

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

/// Config filename
const CONFIG: &str = "config.hcl";
/// Current version
pub const CVERSION: usize = 1;

/// Configuration for the CLI tool, supposed to include parameters and most importantly
/// credentials for the various sources.
///
#[into_configfile]
#[derive(Debug, Default, Deserialize)]
pub struct AcuteConfig {
    /// Each site credentials
    pub site: BTreeMap<String, Auth>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let cfn = opts.config.or(Some(CONFIG.into()));

    // Initialise tracing.
    //
    init_logging(NAME, opts.use_telemetry)?;

    // Config only has the credentials for every source now.
    //
    let cfg = ConfigFile::<Config>::load(cfn.as_deref())?;
    if cfg.inner().version() != CVERSION {
        return Err(Status::BadFileVersion(cfg.inner().version()).into());
    }

    // Banner
    //
    banner()?;

    trace!("Engine starting.");
    // Instantiate Engine
    //
    let mut engine = Engine::new();

    let auth = cfg.inner().unwrap();

    // Load auth data
    //
    engine.auth(&auth.site);

    trace!("Engine initialised and running.");

    let subcmd = &opts.subcmd;
    let res = handle_subcmd(&mut engine, subcmd);
    close_logging();
    res
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
