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
use tracing::debug;

use crate::cli::{Opts, SubCommand};
use crate::cmds::{cmd_clean, cmd_fetch, cmd_find, cmd_show};
use crate::runtime::{finish_runtime, init_runtime};

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
            let _ = cmd_clean(&ctx).await?;
        }
        SubCommand::Fetch => {
            let files = cmd_fetch(&ctx).await?;
            println!("Fetched files:");
            debug!("{}", files.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));
            for file in files {
                println!("  {}", file);
            }
        }
        SubCommand::Find(opts) => {
            println!("Looking for airport: {}", &opts.name);
            let airport_iata = cmd_find(&ctx, &opts.name)?;

            let table = Table::new(&airport_iata).with(Style::sharp()).to_string();
            println!("Found by IATA:\n{table}");
        }
        SubCommand::List(opts) => {
            todo!()
        }
        SubCommand::Show => {
            let _ = cmd_show(&ctx).await?;
        }
    }
    Ok(finish_runtime(&ctx)?)
}
