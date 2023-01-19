use crate::cli::{Opts, SubCommand};
use crate::version::version;

use crate::config::get_config;
use anyhow::Result;
use clap::Parser;
use log::LevelFilter::{Debug, Info, Trace};
use log::{info, trace};

mod cli;
mod cmds;
mod config;
mod version;

fn main() -> Result<()> {
    let opts = Opts::parse();

    println!("{}", version());

    //
    let mut lvl = match opts.verbose {
        0 => Info,
        1 => Debug,
        2 => Trace,
        _ => Trace,
    };

    if opts.debug {
        lvl = Trace;
    }

    stderrlog::new().verbosity(lvl).init()?;

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let cfg = get_config(&opts.config);
    trace!("{:?} db loaded", cfg);

    let subcmd = opts.subcmd;
    match subcmd {
        SubCommand::Import(opts) => todo!(),
        SubCommand::CreateDb(opts) => todo!(),
    }
}
