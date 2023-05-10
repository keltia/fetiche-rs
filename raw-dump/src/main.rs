use std::fs;

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, Parser};
use log::{info, trace};

use raw_dump::{check_args, filter_from_opts, Task};
use raw_dump::{Opts, SubCommand};

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
        SubCommand::Fetch(fopts) => {
            // Fetch data
            //
            trace!("fetch({:?}", fopts);

            check_args(&fopts)?;

            let name = &fopts.site;
            let site = Site::load(name, &cfg)?;
            let filter = filter_from_opts(&fopts)?;

            info!("Fetching from network site {}", name);

            let data = Task::new(name).site(site).with(filter).run()?;
            trace!("{}", data);

            match fopts.output {
                Some(output) => {
                    info!("Writing into {:?}", output);
                    fs::write(output, format!("{:?}", data))?
                }
                _ => println!("{:?}", data),
            }
        }
        SubCommand::Import(opts) => {
            let mut journeys = BTreeMap::<u32, Vec<DronePoint>>::new();

            // Transform into our `Drone` struct and sort it by "journey"
            //
            let data: Vec<Asd> = serde_json::from_str(&data)?;
            let data: Vec<(u32, DronePoint)> = data
                .iter()
                .map(|e| {
                    let d = DronePoint::from(e);
                    (d.journey, d.clone())
                })
                .map(|(j, d)| {
                    let list = match journeys.get_mut(&j) {
                        Some(list) => {
                            list.push(d);
                            list
                        }
                        _ => vec![d],
                    };
                    journeys.update(j, list)
                })
                .collect();

            info!("{} journey points found.", data.len());
        }
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
