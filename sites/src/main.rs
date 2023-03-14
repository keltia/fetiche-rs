use std::path::PathBuf;

use anyhow::Result;
use log::trace;
use log::LevelFilter::{Debug, Trace};

use sites::config::Sites;

fn main() -> Result<()> {
    stderrlog::new().verbosity(Trace).init()?;

    trace!("Loading");
    let sites = Sites::load(&Some(PathBuf::from("./config.hcl")))?;
    dbg!(&sites);
    trace!("Displaying list");
    sites.iter().for_each(|s| println!("{:?}", s));
    Ok(())
}
