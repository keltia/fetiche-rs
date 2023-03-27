//! This is the [Rust] version of `aeroscope.sh` written by Marc Gravis for the ACUTE Project.
//! Now it tries to include features from `aeroscope-CDG.sh` and will support fetching from
//! the Skysafe site as well.
//!
//! It can load from either a server or from a file (easier for offline tests). It uses
//! a configuration file  from `$HOME/.config/drone-utils` or `%LOCALAPPDATA%/drone-utils` on
//! UNIX/Linux and Windows.
//!
//! Our pseudo-Cat21 format-specs is in `format-specs/lib`.
//! The respective format-specs for the other sources are in the files inside the `format-specs` module.
//!
//! Author: Ollivier Robert <ollivier.robert@eurocontrol.int> for the EIH
//! Copyright: (c) 2022 by Ollivier Robert
//!
//! [Rust]: https://rust-lang.org/
//!
mod cli;
mod version;

use crate::cli::{check_args, Opts};
use crate::version::version;

use std::fs;
use std::time::Instant;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use clap::Parser;
use log::{info, trace};
use stderrlog::LogLevelNum::{Debug, Info, Trace};

use cat21conv::Task;
use format_specs::output::{prepare_csv, Cat21};
use format_specs::Format;

use sources::{config::Sites, filter::Filter, site::Site};

/// From the CLI options
///
pub fn filter_from_opts(opts: &Opts) -> Result<Filter> {
    let t: DateTime<Utc> = Utc::now();

    if opts.today {
        // Build our own begin, end
        //
        let begin = NaiveDate::from_ymd_opt(t.year(), t.month(), t.day())
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(t.year(), t.month(), t.day())
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        Ok(Filter::from(begin, end))
    } else if opts.begin.is_some() {
        // Assume both are there, checked elsewhere
        //
        // We have to parse both arguments ourselves because it uses its own format-specs
        //
        let begin = match &opts.begin {
            Some(begin) => NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S")?,
            None => return Err(anyhow!("bad -B parameter")),
        };
        let end = match &opts.end {
            Some(end) => NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S")?,
            None => return Err(anyhow!("Bad -E parameter")),
        };

        Ok(Filter::from(begin, end))
    } else {
        Ok(Filter::default())
    }
}

/// Get the input csv either from the given file or from the network
///
fn get_from_source(cfg: &Sites, opts: &Opts) -> Result<Vec<Cat21>> {
    let fmt = match &opts.format {
        Some(fmt) => fmt.as_str().into(),
        _ => Format::None,
    };

    // Build our filter if needed
    //
    let filter = filter_from_opts(opts)?;

    match &opts.input {
        Some(what) => {
            // Fetch from given file
            //
            info!("Reading from {:?}", what);

            let fname = what
                .to_str()
                .ok_or_else(|| anyhow!("Bad file name {:?}", what))?;

            Task::new(fname).path(fname).format(fmt).run()
        }
        _ => {
            // Fetch from network
            //
            let name = opts
                .site
                .as_ref()
                .ok_or_else(|| anyhow!("Bad site name {:?}", opts.site))?;
            let site = Site::load(name, cfg)?;

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
        .modules(["cat21conv", "format-specs", "sources"])
        .verbosity(lvl)
        .init()?;

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let cfg = Sites::load(&opts.config)?;
    trace!("{} sources loaded", cfg.len());

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
