//! Module dealing with various ACUTE-specific data like our antennas, sites, etc.
//!
//! This provides a CRUD-like interface with subcommands like `add` & `delete`.
//!

use clap::Parser;
use duckdb::arrow::array::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;
use eyre::Result;
use tracing::trace;

pub(crate) use antennas::*;
pub(crate) use install::*;
pub(crate) use sites::*;

use crate::config::Context;

mod antennas;
mod install;
mod sites;

#[derive(Debug, Parser)]
pub struct AcuteOpts {
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: Option<String>,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    #[clap(subcommand)]
    pub subcmd: AcuteSubCommand,
}

#[derive(Debug, Parser)]
pub enum AcuteSubCommand {
    /// Display all antennas.
    Antennas(AntennasOpts),
    /// Fetch which antenna was on a site and when.
    Install(InstOpts),
    /// Display all sites.
    Sites(SiteOpts),
}

// Sub-commands for all the categories.
//

#[derive(Debug, Default, Parser)]
pub enum CrudSubCommand {
    /// Add a something
    Add,
    /// Modify a something
    Modify,
    /// Remove a something
    Remove,
    /// Default is listing everything
    #[default]
    List,
}

// ----- Dispatching

#[tracing::instrument(skip(ctx))]
pub fn run_acute_cmd(ctx: &Context, opts: &AcuteOpts) -> Result<()> {
    trace!("execute");

    let dbh = ctx.db();
    match opts.subcmd {
        AcuteSubCommand::Antennas(_) => {
            // Fetch antennas as Arrow
            //
            let mut stmt = dbh.prepare("select * from antennas;")?;

            println!("Listing all antennas:");
            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            print_batches(&rbs)?;
        }
        AcuteSubCommand::Install(_) => {
            // Find all installations with sites' name and antenna's ID
            //
            let mut stmt = dbh.prepare(
                r##"
SELECT 
  inst.id,
  sites.name,
  start_at,
  end_at,
  antennas.name AS station_name
FROM
  installations AS inst
  JOIN antennas ON antennas.id = inst.antenna_id
  JOIN sites ON inst.site_id = sites.id
ORDER BY start_at
        "##,
            )?;

            println!("Listing all installations:");
            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            print_batches(&rbs)?;
        }
        AcuteSubCommand::Sites(_) => {
            // Fetch sites
            //
            let mut stmt = dbh.prepare(
                r##"
SELECT
  name,
  code,
  ST_Point2D(2.35, 48.6) AS ref,
  ST_Point2D(longitude, latitude) AS here,
  ST_Distance_Spheroid(ref, here) / 1000 AS distance,
FROM sites
ORDER BY
  name
    "##,
            )?;

            println!("Listing all sites:");
            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            print_batches(&rbs)?;
        }
    }

    Ok(())
}
