use anyhow::Result;
use clap::Parser;
use log::LevelFilter::{Debug, Info, Trace};
use log::{info, trace};

use crate::cli::{Opts, SubCommand};
use crate::config::{get_config, DB};
use crate::version::version;

mod cli;
mod cmds;
mod config;
mod version;

fn main() -> Result<()> {
    let opts = Opts::parse();

    println!("{}", version());

    env_logger::init();

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let cfg = get_config(&opts.config);
    trace!("{:?} db loaded", cfg);

    let subcmd = opts.subcmd;
    match subcmd {
        SubCommand::Import(opts) => todo!(),
        SubCommand::CreateDb(opts) => todo!(),
        SubCommand::ListDb => cfg.db.iter().for_each(|(name, db)| println!("{db}")),
    }
    Ok(())
}
