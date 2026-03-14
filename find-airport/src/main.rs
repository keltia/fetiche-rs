//! Search for a specific airport by IATA code/ICAO code/Name, or list a specific country.
//!
//! Source: Use opendata from OurAirports: https://ourairports.com/data/
//!
//! Before doing any search, retrieve data with:
//! ```shell
//! find-airport fetch
//! ```
//!

use std::fmt::Debug;

use clap::{crate_authors, crate_description, Parser};
use csv::Writer;
use eyre::Result;
use serde_json::json;
use tabled::settings::object::Columns;
use tabled::settings::{Alignment, Style};
use tabled::{Table, Tabled};
use tokio::task::spawn_blocking;
use tracing::debug;

use crate::cli::{Format, Opts, SubCommand};
use crate::cmds::{cmd_clean, cmd_fetch, cmd_find, cmd_show, Airport};
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

    if opts.version {
        eprintln!("{} {}", NAME, env!("CARGO_PKG_VERSION"));
        banner();
        return Ok(());
    }

    if !opts.quiet {
        banner();
    }

    let ctx = init_runtime(&opts).await?;

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

            let fmt = if fopts.json {
                Format::Json
            } else if fopts.csv {
                Format::Csv
            } else if fopts.ndjson {
                Format::Ndjson
            } else {
                Format::Plain
            };
            let result = format_result_as(results, fmt)?;
            println!("{}", result);
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

#[tracing::instrument]
fn format_result_as(results: Vec<Airport>, fmt: Format) -> Result<String> {
    Ok(match fmt {
        Format::Json => json!(results).to_string(),
        Format::Csv => {
            let mut wtr = Writer::from_writer(vec![]);

            for airport in results {
                wtr.serialize(airport)?;
            }

            let bytes = wtr.into_inner()?;
            String::from_utf8(bytes)?
        }
        Format::Ndjson => results
            .iter()
            .map(|airport| json!(airport).to_string())
            .collect::<Vec<String>>()
            .join("\n"),
        _ => {
            println!("\nFound by IATA/ICAO/Name/Country:\n");
            display_result_table(results)
        }
    })
}

/// Return our version number
///
#[inline]
pub fn version() -> String {
    USER_AGENT.to_string()
}

/// Display banner
///
fn banner() {
    eprintln!(
        r##"
{} by {}
{}
"##,
        USER_AGENT,
        crate_authors!(),
        crate_description!()
    )
}
