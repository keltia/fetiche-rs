use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{CommandFactory, crate_authors, crate_description, crate_version, Parser};
use clap_complete::generate;
use log::{info, trace};

use acutectl::{
    Config, fetch_from_site, ImportSubCommand, ListSubCommand, Opts, stream_from_site,
    SubCommand,
};
use fetiche_engine::Engine;
use fetiche_formats::Format;
use fetiche_sources::{Flow, Site};

/// Binary name, using a different binary name
pub(crate) const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub(crate) const VERSION: &str = crate_version!();
/// Authors
pub(crate) const AUTHORS: &str = crate_authors!();

fn main() -> Result<()> {
    let opts = Opts::parse();
    let cfn = opts.config.clone();

    // Initialise logging.
    //
    env_logger::init();

    // Config only has the credentials for every source now.
    //
    let cfg = Config::load(cfn)?;

    // Banner
    //
    banner()?;

    let engine = Engine::new();
    let src = engine.sources();

    info!("{:?} sources loaded", src.len());

    let subcmd = &opts.subcmd;
    match subcmd {
        // Handle `fetch site`
        //
        SubCommand::Fetch(fopts) => {
            trace!("fetch");

            fetch_from_site(&engine, fopts)?;
        }

        // Handle `stream site`
        //
        SubCommand::Stream(sopts) => {
            trace!("stream");

            stream_from_site(&engine, sopts)?;
        }

        // Handle `import site`  and `import file`
        // FIXME:
        //
        SubCommand::Import(opts) => {
            trace!("import");

            match &opts.subcmd {
                ImportSubCommand::ImportSite(fopts) => {
                    trace!("drone import site");

                    let srcs = engine.sources();
                    let site = match Site::load(&fopts.site, &srcs)? {
                        Flow::Fetchable(s) => s,
                        _ => return Err(anyhow!("this site is not fetchable")),
                    };
                    let fmt = site.format();

                    // FIXME
                    let data: Vec<u8> = vec![];

                    fetch_from_site(&engine, fopts)?;

                    //import_data(&cfg, &data, fmt)?;
                }
                ImportSubCommand::ImportFile(if_opts) => {
                    trace!("drone import file");

                    let data = fs::read_to_string(&if_opts.file)?;
                    let fmt = Format::from(if_opts.format.clone().unwrap().as_str());

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
            generate(generator, &mut Opts::command(), NAME, &mut io::stdout());
        }

        // Standalone `list` command
        //
        SubCommand::List(lopts) => match lopts.cmd {
            ListSubCommand::Sources => {
                info!("Listing all sources:");

                let str = engine.list_sources()?;
                writeln!(io::stderr(), "{}", str)?;
            }
            ListSubCommand::Formats => {
                info!("Listing all formats:");

                let str = engine.list_formats()?;
                writeln!(io::stderr(), "{}", str)?;
            }
            ListSubCommand::Tokens => {
                info!("Listing all tokens:");

                let str = engine.list_tokens()?;
                writeln!(io::stderr(), "{}", str)?;
            }
        },

        // Standalone `version` command
        //
        SubCommand::Version => {
            eprint!("Modules: ");
            [
                fetiche_engine::version(),
                fetiche_formats::version(),
                fetiche_sources::version(),
            ]
            .iter()
            .for_each(|s| eprint!("{s} "));
        }
    }
    Ok(())
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
