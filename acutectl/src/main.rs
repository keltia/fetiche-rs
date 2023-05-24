use std::fs;
use std::io;
use std::io::Write;

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, CommandFactory, Parser};
use clap_complete::generate;
use log::{info, trace};

use acutectl::{
    fetch_from_site, import_data, list_formats, list_sources, list_tokens, ImportSubCommand,
    ListSubCommand, Opts, SubCommand,
};
use fetiche_formats::Format;
use fetiche_sources::{Site, Sources};

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

    // Read sources
    //
    let cfn = match cfn {
        Some(cfn) => cfn,
        None => Sources::default_file(),
    };

    // Banner
    //
    writeln!(io::stderr(), "{}", version())?;

    // Load default config if nothing is specified
    //
    info!("Loading config from {}…", cfn.to_string_lossy());
    let cfg = Sources::load(&Some(cfn));

    let cfg = match cfg {
        Ok(cfg) => cfg,
        Err(e) => {
            // Early exit if we have an error parsing `sources.hcl`.
            //
            return Err(e);
        }
    };
    info!("{:?} sources loaded", cfg.len());

    let subcmd = &opts.subcmd;
    match subcmd {
        SubCommand::Fetch(fopts) => {
            trace!("fetch");
            let data = fetch_from_site(&cfg, fopts)?;

            match &fopts.output {
                Some(output) => {
                    info!("Writing into {:?}", output);
                    fs::write(output, data)?
                }
                // stdout otherwise
                //
                _ => println!("{}", data),
            }
        }

        // Handle `import site`  and `import file`
        //
        SubCommand::Import(opts) => {
            trace!("import");

            match &opts.subcmd {
                ImportSubCommand::ImportSite(fopts) => {
                    trace!("drone import site");

                    let fmt = Site::load(&fopts.site, &cfg)?.format();

                    let data = fetch_from_site(&cfg, fopts)?;

                    import_data(&cfg, &data, fmt)?;
                }
                ImportSubCommand::ImportFile(if_opts) => {
                    trace!("drone import file");

                    let data = fs::read_to_string(&if_opts.file)?;
                    let fmt = Format::from(if_opts.format.clone().unwrap().as_str());

                    import_data(&cfg, &data, fmt)?;
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

                let str = list_sources(&cfg)?;
                writeln!(io::stderr(), "{}", str)?;
            }
            ListSubCommand::Formats => {
                info!("Listing all formats:");

                let str = list_formats()?;
                writeln!(io::stderr(), "{}", str)?;
            }
            ListSubCommand::Tokens => {
                info!("Listing all tokens:");

                let str = list_tokens()?;
                writeln!(io::stderr(), "{}", str)?;
            }
        },
    }
    Ok(())
}

/// Display our version banner
///
#[inline]
pub fn version() -> String {
    format!(
        "{}/{} by {}\n{}\n",
        NAME,
        VERSION,
        AUTHORS,
        crate_description!()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(version().contains(NAME));
        assert!(version().contains(VERSION));
        assert!(version().contains(AUTHORS))
    }
}
