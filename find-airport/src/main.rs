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
use tabled::settings::object::Columns;
use tabled::settings::{Alignment, Style};
use tabled::Table;
use tracing::debug;

use crate::cli::{Opts, SubCommand};
use crate::cmds::{cmd_clean, cmd_fetch, cmd_find, cmd_show, Work};
use crate::runtime::{finish_runtime, init_runtime};

mod cli;
mod cmds;
mod config;
mod error;
mod runtime;

const NAME: &str = "find-airport";
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let ctx = init_runtime(&opts).await?;

    match &opts.cmd {
        SubCommand::Clean => {
            if !ctx.dry_run {
                let files = cmd_clean(&ctx).await?;
                debug!("files={}", files.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));

                let table = display_work_table(files);
                println!("\nRemoved Files:\n{table}");
            } else {
                println!("Dry run, not removing anything.");
            }
        }
        SubCommand::Fetch => {
            if !ctx.dry_run {
                let files = cmd_fetch(&ctx).await?;
                debug!("files={}", files.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(","));

                let table = display_work_table(files);
                println!("\nFiles:\n{table}");
            } else {
                println!("Dry run, not fetching anything.");
            }
        }
        SubCommand::Find(opts) => {
            println!("Looking for airport: {}", &opts.name);
            let airport_iata = cmd_find(&ctx, &opts.name)?;

            let table = Table::new(&airport_iata).with(Style::sharp()).to_string();
            println!("\nFound by IATA:\n{table}");
        }
        SubCommand::Show => {
            let files = cmd_show(&ctx).await?;
            let table = display_work_table(files);
            println!("\nFiles:\n{table}");
        }
    }
    Ok(finish_runtime(&ctx)?)
}

#[tracing::instrument]
fn display_work_table(list: Vec<Work>) -> String {
    let table = Table::new(list)
        .with(Style::sharp())
        .modify(Columns::one(3), Alignment::right())
        .modify(Columns::one(4), Alignment::right())
        .to_string();
    table
}

