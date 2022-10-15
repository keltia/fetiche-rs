mod config;
mod fetch;
mod process;
mod version;

use crate::config::{get_config, Config};
use crate::fetch::fetch_csv;
use crate::process::{Cat21, prepare_csv, process_data};
use crate::version::version;

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use csv::ReaderBuilder;

pub struct Context {
    pub cfg: Config,
    pub client: reqwest::blocking::Client,
}

/// CLI options
#[derive(Parser, Debug)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version ! (), author = crate_authors ! ())]
struct Opts {
    /// configuration file
    #[clap(short = 'c', long)]
    config: Option<PathBuf>,
    /// debug mode
    #[clap(short = 'D', long = "debug")]
    debug: bool,
    /// Output file
    #[clap(short = 'o', long)]
    output: Option<PathBuf>,
    /// Verbose mode
    #[clap(short = 'v', long)]
    verbose: bool,
    #[clap(short = 'V', long)]
    version: bool,
    /// Input file
    input: Option<PathBuf>,
}

/// Get the input csv either from the given file or from the network
///
fn get_from_source(ctx: &Context, what: Option<PathBuf>) -> Result<Vec<Cat21>> {
    match what {
        Some(what) => {
            // Fetch from given file
            //
            let mut rdr = ReaderBuilder::new().from_path(what)?;
            process_data(&mut rdr)
        },
        _ => {
            // Fetch from network
            //
            let res = fetch_csv(ctx)?;
            let mut rdr = ReaderBuilder::new().from_reader(res.as_bytes());
            process_data(&mut rdr)
        },
    }
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Bypass
    //
    if opts.version {
        println!("{}", version());
        return Ok(());
    }

    // Load default config if nothing is specified
    let cfg = get_config(opts.config);

    let ctx = Context {
        client: reqwest::blocking::Client::new(),
        cfg,
    };

    // Load data from original csv or site
    //
    let what = opts.input;

    let data = get_from_source(&ctx, what)?;
    let data = prepare_csv(data)?;

    match opts.output {
        Some(output) => fs::write(output, data)?,
        _ => println!("{}", data),
    }

    Ok(())
}
