use std::fs;
use std::io;

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, CommandFactory, Parser};
use clap_complete::generate;
use log::{info, trace};

use acutectl::{fetch_from_site, import_data, DroneSubCommand};
use acutectl::{ImportSubCommand, Opts, SubCommand};
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
    println!("{}", version());

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let cfg = Sites::load(&Some(cfn))?;
    info!("{:?} sources loaded", cfg.len());

    let subcmd = opts.subcmd;
    match subcmd {
        // Handle `fetch`
        //
        SubCommand::Adsb(aopts) => {
            unimplemented!()
        }
        SubCommand::Drone(dopts) => {
            match dopts.subcmd {
                DroneSubCommand::Fetch(fopts) => {
                    let data = fetch_from_site(&cfg, &fopts)?;

                    match &fopts.output {
                        Some(output) => {
                            info!("Writing into {:?}", output);
                            fs::write(output, data)?
                        }
                        /// stdout otherwise
                        ///
                        _ => println!("{:?}", data),
                    }
                }
                // Handle `import site`  and `import file`
                //
                DroneSubCommand::Import(opts) => match opts.subcmd {
                    ImportSubCommand::ImportSite(fopts) => {
                        let fmt = Site::load(&fopts.site, &cfg)?;
                        let fmt = fmt.format();

                        let data = fetch_from_site(&cfg, &fopts)?;

                        import_data(&cfg, &data, fmt)?;
                    }
                    ImportSubCommand::ImportFile(if_opts) => {
                        let data = fs::read_to_string(if_opts.file)?;
                        let fmt = Format::from(if_opts.format.unwrap().as_str());

                            import_data(&cfg, &data, fmt)?;
                        }
                    }
                }
            }
        }
        SubCommand::Completion(copts) => {
            let generator = copts.shell;
            generate(generator, &mut Opts::command(), NAME, &mut io::stdout());
        }

        // Standalone `list` command
        //
        SubCommand::List => {
            info!("Listing all sources:");
            cfg.iter()
                .for_each(|(name, site)| println!("{} = {}", name, site))
        }
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
