//! This is the [Rust] version of `aeroscope.sh` written by Marc Gravis for the ACUTE Project.
//! Now it tries to include features from `aeroscope-CDG.sh` and will support fetching from
//! the Skysafe site as well.
//!
//! It can load from either a server or from a file (easier for offline tests). It uses
//! a configuration file  from `$HOME/.config/drone-utils` or `%LOCALAPPDATA%/drone-utils` on
//! UNIX/Linux and Windows.
//!
//! Our pseudo-Cat21 format is in `format/mod.rs`.
//! The respective format for the other sources are in the files inside the `format` module.
//!
//! Author: Ollivier Robert <ollivier.robert@eurocontrol.int> for the EIH
//! Copyright: (c) 2022 by Ollivier Robert
//!
//! [Rust]: https://rust-lang.org/
//!
mod cli;
mod version;

use drone_utils::config::{get_config, Config};
use drone_utils::filter::Filter;
use drone_utils::format::{prepare_csv, Cat21, Format};
use drone_utils::site::Site;
use drone_utils::task::Task;

use crate::cli::{check_args, Opts};
use crate::version::version;

use std::fs;
use std::time::Instant;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use clap::Parser;
use log::{info, trace};
use stderrlog::LogLevelNum::{Debug, Info, Trace};

/// From the CLI options
///
pub fn filter_from_opts(opts: &Opts) -> Filter {
    let t: DateTime<Utc> = Utc::now();

    if opts.today {
        // Build our own begin, end
        //
        let begin = NaiveDate::from_ymd(t.year(), t.month(), t.day()).and_hms(0, 0, 0);
        let end = NaiveDate::from_ymd(t.year(), t.month(), t.day()).and_hms(23, 59, 59);

        Filter::from(begin, end)
    } else if opts.begin.is_some() {
        // Assume both are there, checked elsewhere
        //
        Filter::from(opts.begin.unwrap(), opts.end.unwrap())
    } else {
        Filter::default()
    }
}

/// Get the input csv either from the given file or from the network
///
fn get_from_source(cfg: &Config, opts: &Opts) -> Result<Vec<Cat21>> {
    let fmt = match &opts.format {
        Some(fmt) => fmt.as_str().into(),
        _ => Format::None,
    };

    // Build our filter if needed
    //
    let filter = filter_from_opts(opts);

    match &opts.input {
        Some(what) => {
            // Fetch from given file
            //
            info!("Reading from {:?}", what);

            let fname = what.to_str().unwrap();

            Task::new(fname).path(fname).format(fmt).run()
        }
        _ => {
            // Fetch from network
            //
            let name = opts.site.as_ref().unwrap();
            let site = Site::new().load(name, cfg)?;

            info!("Fetching from network site {}", name);

            Task::new(name).site(site).with(filter).run()
        }
    }
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Add banner
    //
    println!("{}\n", version());

    // Exit if needed
    //
    if opts.version {
        return Ok(());
    }

    // Check arguments
    //
    check_args(&opts)?;

    // Check verbosity
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

    // Prepare logging.
    //
    stderrlog::new()
        //.modules([module_path!(), "drone-utils", "format", "site"])
        .verbosity(lvl)
        .init()?;

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let cfg = get_config(&opts.config);
    trace!("{} sites loaded", cfg.sites.len());

    info!("Loading data…");

    let now = Instant::now();

    let data = get_from_source(&cfg, &opts)?;
    let len = data.len();
    trace!("{} bytes received", len);
    let data = prepare_csv(data)?;

    let now = now.elapsed().as_millis();

    info!("Generating csv…");
    match opts.output {
        Some(output) => fs::write(output, data)?,
        _ => println!("{}", data),
    }

    info!(
        "{} lines processed in {}ms: {} lines/s",
        len,
        now,
        (len as u128 / now * 1000)
    );

    Ok(())
}
