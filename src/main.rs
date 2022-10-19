//! This is the [Rust] version of `aeroscope.sh` write by Marc Gravis for the ACUTE Project.
//!
//! It can load from either the Aeroscope server or from a file (easier for offline tests). It uses
//! a configuration file  from `$HOME/.config/drone-gencsv` or `%LOCALAPPDATA%/drone-gencsv` on
//! UNIX/Linux and Windows.
//!
//! The mapping from the Aeroscope CSV to the pseudo-Cat21 format is in `process.rs`.
//!
//! Author: Ollivier Robert <ollivier.robert@eurocontrol.int> for the EIH
//! Copyright: (c) 2022 by Ollivier Robert
//!
//! [Rust]: https://rust-lang.org/
//!
mod cli;
mod config;
mod fetch;
mod process;
mod version;

use crate::cli::Opts;
use crate::config::{get_config, Config};
use crate::fetch::fetch_csv;
use crate::process::{prepare_csv, process_data, Cat21};
use crate::version::version;

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use csv::ReaderBuilder;
use log::info;
use stderrlog::LogLevelNum::Trace;

#[derive(Debug)]
pub struct Context {
    /// Config taken from `config.toml`, modified by flags.
    pub cfg: Config,
    /// We want to restrict ourselves to today's data
    pub today: bool,
    /// Begin date
    pub begin: Option<DateTime<Utc>>,
    /// Begin date
    pub end: Option<DateTime<Utc>>,
    /// We want to reuse the HTTP client
    pub client: reqwest::blocking::Client,
}

/// Get the input csv either from the given file or from the network
///
fn get_from_source(ctx: &Context, what: Option<PathBuf>) -> Result<Vec<Cat21>> {
    match what {
        Some(what) => {
            // Fetch from given file
            //
            info!("Reading from {:?}", what.to_str().unwrap());
            let mut rdr = ReaderBuilder::new().flexible(true).from_path(what)?;
            process_data(&mut rdr)
        }
        _ => {
            // Fetch from network
            //
            info!("Fetching from {}", ctx.cfg.base_url);
            let res = fetch_csv(ctx)?;
            let mut rdr = ReaderBuilder::new()
                .flexible(true)
                .from_reader(res.as_bytes());
            process_data(&mut rdr)
        }
    }
}

fn new_context(opts: &Opts, cfg: Config) -> Context {
    // Create our context
    //
    let mut ctx = Context {
        client: reqwest::blocking::Client::new(),
        today: false,
        begin: opts.begin,
        end: opts.end,
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

    stderrlog::new()
        .module(module_path!())
        .verbosity(Trace)
        .init()?;

    // Load default config if nothing is specified
    //
    info!("Loading config…");
    let mut cfg = get_config(&opts.config);
    trace!("{} sites loaded", cfg.sites.len());

    let ctx = new_context(&opts, cfg);

    // Load data from original csv or site
    //
    let what = opts.input;

    info!("Loading data…");

    let now = Instant::now();

    let data = get_from_source(&ctx, what)?;
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
