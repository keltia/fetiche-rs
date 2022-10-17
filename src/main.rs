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
mod config;
mod fetch;
mod process;
mod version;

use crate::config::{get_config, Config};
use crate::fetch::fetch_csv;
use crate::process::{prepare_csv, process_data, Cat21};
use crate::version::version;

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use csv::ReaderBuilder;
use log::info;
use stderrlog::LogLevelNum::Trace;

#[derive(Debug)]
pub struct Context {
    /// Config taken from `config.toml`, modified by flags.
    pub cfg: Config,
    /// We want to restrict ourselves to today's data
    pub today: bool,
    /// We want to reuse the HTTP client
    pub client: reqwest::blocking::Client,
}

/// CLI options
#[derive(Parser, Debug)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    /// configuration file.
    #[clap(short = 'c', long)]
    config: Option<PathBuf>,
    /// debug mode.
    #[clap(short = 'D', long = "debug")]
    debug: bool,
    /// Output file.
    #[clap(short = 'o', long)]
    output: Option<PathBuf>,
    /// Optional password.
    #[clap(short = 'P', long)]
    password: Option<String>,
    /// We want today only
    #[clap(long)]
    today: bool,
    /// Optional username for the server API.
    #[clap(short = 'U', long)]
    username: Option<String>,
    /// Verbose mode.
    #[clap(short = 'v', long)]
    verbose: Option<usize>,
    /// Display utility full version.
    #[clap(short = 'V', long)]
    version: bool,
    /// Input file.
    input: Option<PathBuf>,
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
    let mut cfg = get_config(opts.config);

    // Allow overriding credentials on CLI (not safe)
    //
    if let Some(login) = opts.username {
        cfg.login = login;
    }
    if let Some(password) = opts.password {
        cfg.password = password;
    }

    // Create our context
    //
    let ctx = Context {
        client: reqwest::blocking::Client::new(),
        today: false,
        cfg,
    };

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
