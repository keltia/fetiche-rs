//! Module dealing with various ACUTE-specific data like our antennas, sites, etc.
//!
//! This provides a CRUD-like interface with subcommands like `add` & `delete`.
//!

use clap::Parser;
use clickhouse::Row;
use eyre::Result;
use serde::{Deserialize, Serialize};
use time::Date;
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
pub async fn run_acute_cmd(ctx: &Context, opts: &AcuteOpts) -> Result<()> {
    trace!("run_acute_cmd");

    let dbh = ctx.db();
    match opts.subcmd {
        // List all antennas
        //
        AcuteSubCommand::Antennas(_) => {
            #[derive(Debug, Deserialize, Serialize, Row)]
            struct Antenna {
                pub id: u32,
                #[serde(rename = "type")]
                pub atype: String,
                pub name: String,
                pub owned: bool,
                pub description: String,
            }

            // Fetch antennas as Arrow
            //
            let res = dbh
                .query("select * from antennas")
                .fetch_all::<Antenna>()
                .await?;

            println!("Listing all antennas:");
            print_batches(&res)?;
        }
        // List all installations
        //
        AcuteSubCommand::Install(_) => {
            #[derive(Debug, Deserialize, Serialize, Row)]
            struct Install {
                pub id: u32,
                pub name: String,
                pub start_at: Date,
                pub end_at: Date,
                pub station_name: String,
                pub comment: String,
            }

            // Find all installations with sites' name and antenna's ID
            //
            let r = r##"
SELECT 
  inst.id,
  sites.name,
  start_at,
  end_at,
  antennas.name AS station_name
  inst.comment,
FROM
  installations AS inst
  JOIN antennas ON antennas.id = inst.antenna_id
  JOIN sites ON inst.site_id = sites.id
ORDER BY start_at
        "##;

            println!("Listing all installations:");
            let rbs = dbh.query(r).fetch_all::<Install>().await?;

            dbg!(&rbs)?;
        }
        AcuteSubCommand::Sites(_) => {

            #[derive(Debug, Deserialize, Serialize, Row)]
            struct Site {
                pub id: u32,
                pub name: String,
                pub code: String,
                pub home: f64,
                pub here: f64,
                pub distance: f64,
            }


            // Fetch sites
            //
            let res: Vec<Site> = dbh.query(
                r##"
SELECT
  id,
  name,
  code,
  longitude,
  latitude,
  ref_altitude,
  ST_Distance_Spheroid(ref, here) / 1000 AS distance,
FROM sites
ORDER BY
  name
    "##).fetch_all::<Site>().await?;

            println!("Listing all sites:");
            dbg!(&res);
        }
    }

    Ok(())
}
