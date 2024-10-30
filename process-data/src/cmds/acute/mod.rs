//! Module dealing with various ACUTE-specific data like our antennas, sites, etc.
//!
//! This provides a CRUD-like interface with subcommands like `add` & `delete`.
//!

use clap::Parser;
use eyre::Result;
use geo::coord;
use klickhouse::{DateTime, QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::read_to_string;
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

    let dbh = ctx.db().await;
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
                .query_collect::<Antenna>("SELECT * FROM antennas")
                .await?;

            println!("Listing all antennas:");
            let res = json!(&res).to_string();
            println!("{res}");
        }
        // List all installations
        //
        AcuteSubCommand::Install(_) => {
            #[derive(Debug, Deserialize, Serialize, Row)]
            struct Install {
                pub id: i32,
                pub name: String,
                pub start_at: DateTime,
                pub end_at: DateTime,
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
    antennas.name AS station_name,
    inst.comment
FROM installations AS inst
INNER JOIN antennas ON antennas.id = inst.antenna_id
INNER JOIN sites ON inst.site_id = sites.id
ORDER BY start_at ASC
INTO OUTFILE '/tmp/installations.txt' AND STDOUT
FORMAT Pretty
           "##;

            eprintln!("Listing all installations:");
            let _ = dbh.execute(r).await?;
            let res = read_to_string("/tmp/installations.txt")?;
            println!("{res}");
        }
        AcuteSubCommand::Sites(_) => {
            #[derive(Debug, Deserialize, Serialize, Row)]
            struct Site {
                pub id: i32,
                pub name: String,
                pub code: String,
                pub home: f32,
                pub here: f32,
                pub distance: f32,
            }

            // This is our current location in Brétigny
            //
            let home = coord! {x: 48.600052, y:2.347038};

            // Fetch sites
            //
            let r = r##"
SELECT
  id,
  name,
  code,
  longitude,
  latitude,
  ref_altitude,
  floor(dist_2d($1, $2, longitude, latitude) / 1000.) AS distance_km
FROM
  sites
ORDER BY
  name
    "##;
            let q = QueryBuilder::new(r).arg(home.y).arg(home.x);
            let res = dbh.query_collect::<Site>(q).await?;

            println!("Listing all sites:");
            let res = json!(&res).to_string();
            println!("{res}");
        }
    }

    Ok(())
}
