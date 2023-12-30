//! This is the [Rust] version of `aeroscope.sh` written by Marc Gravis for the ACUTE Project.
//!
//! Now it tries to include features from `aeroscope-CDG.sh` and will support fetching from
//! the Skysafe site as well.
//!
//! NOTE: Deprecated by the `acutectl` utility which supports more formats and sources.
//!
//! It can load from either a server or from a file (easier for offline tests). It uses
//! a configuration file  from `$HOME/.config/drone-utils` or `%LOCALAPPDATA%/drone-utils` on
//! UNIX/Linux and Windows.
//!
//! Our pseudo-Cat21 formats is in `formats/lib`.
//! The respective formats for the other sources are in the files inside the `formats` module.
//!
//! Author: Ollivier Robert <ollivier.robert@eurocontrol.int> for the EIH
//! Copyright: (c) 2022, 2023, 2024 by Ollivier Robert
//!
//! [Rust]: https://rust-lang.org/
//!
use std::fs;
use std::str::FromStr;
use std::time::Instant;

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use clap::Parser;
use eyre::{eyre, Result};
use tracing::{info, trace};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use cat21conv::Task;
use fetiche_formats::{prepare_csv, Cat21, Format};
use fetiche_sources::{Filter, Flow, Site, Sources};

use crate::cli::{check_args, Opts};
use crate::version::version;

mod cli;
mod version;

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
            .unwrap()
            .and_utc();
        let end = NaiveDate::from_ymd_opt(t.year(), t.month(), t.day())
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap()
            .and_utc();

        Ok(Filter::interval(begin, end))
    } else if opts.begin.is_some() {
        // Assume both are there, checked elsewhere
        //
        // We have to parse both arguments ourselves because it uses its own formats
        //
        let begin = match &opts.begin {
            Some(begin) => NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S")?.and_utc(),
            None => return Err(eyre!("bad -B parameter")),
        };
        let end = match &opts.end {
            Some(end) => NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S")?.and_utc(),
            None => return Err(eyre!("Bad -E parameter")),
        };

        Ok(Filter::interval(begin, end))
    } else if opts.keyword.is_some() {
        let keyword = opts.keyword.clone().unwrap();

        let v: Vec<_> = keyword.split(':').collect();
        let (k, v) = (v[0], v[1]);
        Ok(Filter::Keyword {
            name: k.to_string(),
            value: v.to_string(),
        })
    } else if opts.since.is_some() {
        let d = opts.since.unwrap();

        Ok(Filter::Duration(d))
    } else {
        Ok(Filter::default())
    }
}

/// Get the input csv either from the given file or from the network
///
fn get_from_source(cfg: &Sources, opts: &Opts) -> Result<Vec<Cat21>> {
    let fmt = match &opts.format {
        Some(fmt) => Format::from_str(fmt)?,
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
                .ok_or_else(|| eyre!("Bad file name {:?}", what))?;

            Task::new(fname).path(fname).format(fmt).run()
        }
        _ => {
            // Fetch from network
            //
            let name = opts
                .site
                .as_ref()
                .ok_or_else(|| eyre!("Bad site name {:?}", opts.site))?;
            let site = match Site::load(name, cfg)? {
                Flow::Fetchable(s) => s,
                _ => return Err(eyre!("this site is not fetchable")),
            };

            info!("Fetching from network site {}", name);

            Task::new(name).site(site).when(filter).run()
        }
    }
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Add banner
    //
    eprintln!("{}\n", version());

    // Exit if needed
    //
    if opts.version {
        return Ok(());
    }

    // Check arguments
    //
    check_args(&opts)?;

    // Initialise logging.
    //
    let fmt = fmt::layer()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(false)
        .compact();

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry().with(filter).with(fmt).init();

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let cfg = Sources::load(&opts.config)?;
    trace!("{} sources loaded", cfg.len());

    info!("Loading data…");

    let now = Instant::now();

    let data = get_from_source(&cfg, &opts)?;
    let len = data.len();
    trace!("{} bytes received", len);
    let data = prepare_csv(data, true)?;

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
