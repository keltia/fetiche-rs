//! This is the [Rust] version of `aeroscope.sh` write by Marc Gravis for the ACUTE Project.
//! Now it tries to include features from `aeroscope-CDG.sh` and will support fetching from
//! the Skysafe site as well.
//!
//! It can load from either a server or from a file (easier for offline tests). It uses
//! a configuration file  from `$HOME/.config/drone-gencsv` or `%LOCALAPPDATA%/drone-gencsv` on
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
mod config;
mod format;
mod site;
mod task;
mod version;

use crate::cli::Opts;
use crate::config::{get_config, Config};
use crate::format::{prepare_csv, Cat21, Source};
use crate::site::Site;
use crate::task::Task;
use crate::version::version;

use std::fs;
use std::time::Instant;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use log::{info, trace};
use stderrlog::LogLevelNum::Trace;

#[derive(Debug)]
pub struct Context {
    /// Config taken from `config.toml`, modified by flags.
    pub cfg: Config,
    /// We want to restrict ourselves to today's data
    pub today: bool,
    /// Source to fetch data from
    pub site: Option<String>,
    /// Begin date
    pub begin: Option<DateTime<Utc>>,
    /// Begin date
    pub end: Option<DateTime<Utc>>,
}

/// Get the input csv either from the given file or from the network
///
fn get_from_source(cfg: &Config, opts: &Opts) -> Result<Vec<Cat21>> {
    let fmt = match &opts.format {
        Some(fmt) => Source::from_str(fmt),
        _ => Source::None,
    };

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
            let site = Site::new(cfg, name);

            info!("Fetching from network site {}", name);

            Task::new(name).with(site).run()
        }
    }
}

/// Currently unused
fn new_context(opts: &Opts, cfg: Config) -> Context {
    // Create our context
    //
    let mut ctx = Context {
        today: false,
        begin: opts.begin,
        end: opts.end,
        site: None,
        cfg,
    };

    // Consistency check and update context
    //
    if opts.today {
        let now: DateTime<Utc> = Utc::now();
        let begin = Utc.ymd(now.year(), now.month(), now.day()).and_hms(0, 0, 0);
        let end = Utc
            .ymd(now.year(), now.month(), now.day())
            .and_hms(23, 59, 59);
        ctx.begin = Some(begin);
        ctx.end = Some(end);
    }
    ctx
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

    // Prepare logging.
    //
    stderrlog::new()
        .module(module_path!())
        .verbosity(Trace)
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
