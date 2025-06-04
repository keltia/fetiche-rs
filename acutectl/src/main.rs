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
use eyre::Result;
use serde::Deserialize;
use tracing::{debug, trace};

use acutectl::{handle_subcmd, ConfigCmd, Opts, Status, SubCommand};
use fetiche_common::{close_logging, init_logging, ConfigFile, IntoConfig, Versioned};
use fetiche_engine::Engine;
use fetiche_macros::into_configfile;

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

/// Config filename
const CONFIG: &str = "acutectl.hcl";
/// Current version
pub const CVERSION: usize = 2;

#[allow(dead_code)]
/// Configuration for the CLI tool, supposed to include parameters
///
#[into_configfile(version = 2, filename = "acutectl.hcl")]
#[derive(Debug, Default, Deserialize)]
pub struct AcuteConfig {
    use_async: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();
    let cfn = opts.config.or(Some(CONFIG.into()));

    // Initialise tracing.
    //
    init_logging(NAME, opts.use_telemetry, opts.use_tree, opts.use_file)?;

    // Config only has the credentials for every source now.
    //
    let cfile = ConfigFile::<AcuteConfig>::load(cfn.as_deref())?;
    debug!("cfile = {:?}", cfile);

    let cfg = cfile.inner();
    if cfg.version() != CVERSION {
        return Err(Status::BadFileVersion(cfg.version()).into());
    }

    // Banner
    //
    if !opts.quiet {
        banner();
    }

    trace!("Engine starting.");
    // Instantiate Engine
    //
    let mut engine = Engine::single().await;

    trace!("Engine initialised and running.");

    // initialise signal handling
    //
    let mut e = engine.clone();
    let _ = ctrlc::set_handler(move || {
        trace!("Ctrl-C pressed");
        e.shutdown();
        close_logging();
        std::process::exit(1);
    });

    let subcmd = opts.subcmd;

    // We shortcut the `config`  sub-commands here to avoid exporting some variables to `handle_subcmd()`
    //
    match subcmd {
        // Handle `config acutectl|engine|sources`
        //
        SubCommand::Config(copts) => match copts.subcmd {
            ConfigCmd::Acutectl => {
                let p = cfile
                    .config_path()
                    .join(CONFIG)
                    .to_string_lossy()
                    .to_string();
                println!("{p}");
            }
            ConfigCmd::Engine => {
                let p = engine.config_file().to_string_lossy().to_string();
                println!("{p}");
            }
            ConfigCmd::Sources => {
                let p = engine.sources_file().to_string_lossy().to_string();
                println!("{p}");
            }
        },
        _ => {
            // For the moment the whole of Engine is sync so we need to block.
            //
            let _ = handle_subcmd(&mut engine, &subcmd).await?;
        }
    }
    close_logging();
    Ok(())
}

/// Return our version number
///
#[inline]
pub fn version() -> String {
    format!("{NAME}/{VERSION}")
}

/// Display banner
///
fn banner() {
    eprintln!(
        r##"
{} by {AUTHORS}
{}
"##,
        version(),
        crate_description!()
    )
}
