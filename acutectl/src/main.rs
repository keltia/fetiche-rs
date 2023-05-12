use std::fs;
use std::io;
use std::io::Write;

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, CommandFactory, Parser};
use clap_complete::generate;
use log::{info, trace};

use acutectl::{
    fetch_from_site, import_data, list_formats, list_sources, DroneSubCommand, ImportSubCommand,
    ListSubCommand, Opts, SubCommand,
};
use format_specs::Format;
use sources::{Site, Sites};

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
        None => Sites::default_file(),
    };

    // Banner
    //
    writeln!(io::stderr(), "{}", version())?;

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let cfg = Sites::load(&Some(cfn))?;
    info!("{:?} sources loaded", cfg.len());

    let subcmd = &opts.subcmd;
    match subcmd {
        // Handle `adsb` commands
        //
        SubCommand::Adsb(aopts) => {
            trace!("adsb");

            unimplemented!()
        }
        // Handle `drone` commands
        //
        SubCommand::Drone(dopts) => {
            trace!("drone");

            match &dopts.subcmd {
                DroneSubCommand::Fetch(fopts) => {
                    trace!("drone fetch");
                    let data = fetch_from_site(&cfg, &fopts)?;

                    match &fopts.output {
                        Some(output) => {
                            info!("Writing into {:?}", output);
                            fs::write(output, data)?
                        }
                        // stdout otherwise
                        //
                        _ => println!("{:?}", data),
                    }
                }
                // Handle `import site`  and `import file`
                //
                DroneSubCommand::Import(opts) => {
                    trace!("drone import");

                    match &opts.subcmd {
                        ImportSubCommand::ImportSite(fopts) => {
                            trace!("drone import site");

                            let fmt = Site::load(&fopts.site, &cfg)?.format();

                            let data = fetch_from_site(&cfg, &fopts)?;

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
                info!("Listing all formats!");

                let str = list_formats()?;
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
