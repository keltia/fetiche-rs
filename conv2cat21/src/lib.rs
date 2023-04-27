//! Conv2cat21
//!
//! Application based on the [Abscissa] framework.
//!
//! [Abscissa]: https://github.com/iqlusioninc/abscissa
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

// Tip: Deny warnings with `RUSTFLAGS="-D warnings"` environment variable in CI

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]

pub mod application;
pub mod commands;
pub mod config;
pub mod error;
pub mod prelude;
pub mod task;

use std::fs;
use std::time::Instant;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use clap::Parser;
use log::{info, trace};
use stderrlog::LogLevelNum::{Debug, Error, Info, Trace};

use crate::commands::EntryPoint;
use crate::task::Task;
use format_specs::{prepare_csv, Cat21, Format};
use sources::{Filter, Site, Sites};

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

/// Check the presence and validity of some of the arguments
///
pub fn check_args(opts: &Opts) -> Result<()> {
    // Check arguments.
    //
    if opts.input.is_some() && opts.site.is_some() {
        return Err(anyhow!("Specify either a site or a filename, not both"));
    }

    if opts.input.is_none() && opts.site.is_none() {
        return Err(anyhow!("Specify at least a site or a filename"));
    }

    if opts.input.is_some() && opts.format.is_none() {
        return Err(anyhow!("Format must be specified for files"));
    }

    // Do we have options for filter

    if opts.today && (opts.begin.is_some() || opts.end.is_some()) {
        return Err(anyhow!("Can not specify --today and -B/-E"));
    }

    if (opts.begin.is_some() && opts.end.is_none()) || (opts.begin.is_none() && opts.end.is_some())
    {
        return Err(anyhow!("We need both -B/-E or none"));
    }

    Ok(())
}

fn realmain(opts: &EntryPoint) -> Result<()> {
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
    check_args(opts)?;

    // Check verbosity
    //

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
