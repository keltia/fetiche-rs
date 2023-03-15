use std::path::PathBuf;

use anyhow::Result;
use log::LevelFilter::{Debug, Trace};
use log::{info, trace};

use sites::config::Sites;

fn main() -> Result<()> {
    stderrlog::new().verbosity(Trace).init()?;

    info!("Loading");
    let sites = Sites::load(&Some(PathBuf::from("./config.hcl")))?;
    trace!("Displaying list");
    sites.iter().for_each(|s| println!("{:?}", s));
    Ok(())
}
