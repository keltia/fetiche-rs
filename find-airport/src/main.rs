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
use std::fmt::Debug;
use tabled::settings::object::Columns;
use tabled::settings::{Alignment, Style};
use tabled::{Table, Tabled};
use tokio::task::spawn_blocking;
use tracing::debug;

use crate::cli::{Opts, SubCommand};
use crate::cmds::{cmd_clean, cmd_fetch, cmd_find, cmd_show};
use crate::runtime::{finish_runtime, init_runtime, Context};

mod cli;
mod cmds;
mod config;
mod error;
mod runtime;

const NAME: &str = env!("CARGO_PKG_NAME");
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let ctx = init_runtime(&opts).await?;

    println!("{USER_AGENT}\n");
    println!("Repository: {}", repo_path(&ctx));
    match &opts.cmd {
        SubCommand::Clean => {
            if !ctx.dry_run {
                let files = cmd_clean(&ctx).await?;
                debug!(
                    "files={}",
                    files
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                );

                let table = display_result_table(files);
                println!("\nRemoved Files:\n{table}");
            } else {
                println!("Dry run, not removing anything.");
            }
        }
        SubCommand::Fetch => {
            if !ctx.dry_run {
                let files = cmd_fetch(&ctx).await?;
                debug!(
                    "files={}",
                    files
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                );

                let table = display_result_table(files);
                println!("\nFiles:\n{table}");
            } else {
                println!("Dry run, not fetching anything.");
            }
        }
        SubCommand::Find(fopts) => {
            println!("Looking for airport: {}", &fopts.text);

            // polars is not async-friendly, when using lazy frames
            // cf.https://stackoverflow.com/questions/77294105/how-do-i-call-the-polars-rust-api-from-an-async-function#77312986
            //
            let ctx1 = ctx.clone();
            let opts1 = fopts.clone();
            let airport_iata = spawn_blocking(move || cmd_find(&ctx1, &opts1)).await?;

            let results = match airport_iata {
                Ok(airport_iata) => {
                    debug!("res={:?}", airport_iata);

                    airport_iata
                }
                Err(e) => {
                    eprintln!("Error finding airport: {}", e.to_string());
                    return Err(e);
                }
            };

            let table = display_result_table(results);
            println!("\nFound by IATA/ICAO/Name/Country:\n{table}");
        }
        SubCommand::Show => {
            let files = cmd_show(&ctx).await?;
            let table = display_result_table(files);
            println!("\nFiles:\n{table}");
        }
    }
    Ok(finish_runtime(&ctx)?)
}

#[tracing::instrument]
fn display_result_table<T>(list: Vec<T>) -> String
where
    T: Debug + Tabled,
{
    let table = Table::new(list)
        .with(Style::sharp())
        .modify(Columns::one(2), Alignment::right())
        .modify(Columns::one(3), Alignment::right())
        .modify(Columns::one(4), Alignment::right())
        .modify(Columns::one(5), Alignment::center())
        .modify(Columns::one(7), Alignment::center())
        .to_string();
    table
}

#[tracing::instrument]
fn repo_path(ctx: &Context) -> String {
    let repo_path = ctx.cfg["datalake"].clone();
    format!("{}/files", repo_path)
}
