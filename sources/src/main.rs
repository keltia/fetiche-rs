use std::path::PathBuf;

use anyhow::Result;
use log::LevelFilter::Trace;
use log::{info, trace};

use sources::Sites;

fn main() -> Result<()> {
    stderrlog::new().verbosity(Trace).init()?;

    info!("Loading");
    let sites = Sites::load(&Some(PathBuf::from("./config.hcl")))?;
    trace!("Displaying list");
    sites.iter().for_each(|s| println!("{:?}", s));
    Ok(())
}
