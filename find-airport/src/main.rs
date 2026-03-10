//! Try find a specific airport by IATA code.
//!
//! Use opendata from OurAirports: https://ourairports.com/data/
//!
//! Get the csv file with curl, and convert into parquet:
//! ```text
//! curl -O  https://davidmegginson.github.io/ourairports-data/airports.csv
//! bdt convert -s airports.csv airports.parquet
//! ```
//!
use clap::Parser;
use eyre::Result;
use tabled::settings::Style;
use tabled::{Table, Tabled};

use crate::airport::find_airport;
use crate::cli::{Opts, SubCommand};
use crate::runtime::{finish_runtime, init_runtime};

use crate::cmds::{clean, fetch, show};

mod airport;
mod cli;
mod cmds;
mod config;
mod error;
mod runtime;

const NAME: &str = "find-airport";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let ctx = init_runtime(&opts).await?;

    match &opts.cmd {
        SubCommand::Clean => {
            let _ = clean(&ctx).await?;
        }
        SubCommand::Fetch => {
            let files = fetch(&ctx).await?;
            println!("Fetched {} entries", files.len());
        }
        SubCommand::Find(opts) => {
            println!("Looking for airport: {}", &opts.name);
            let airport_iata = find_airport(&ctx, &opts.name)?;

            let table = Table::new(&airport_iata).with(Style::sharp()).to_string();
            println!("Found by IATA:\n{table}");
        }
        SubCommand::List(opts) => {
            todo!()
        }
        SubCommand::Show => {
            let _ = show(&ctx).await?;
        }
    }
    Ok(finish_runtime(&ctx)?)
}
