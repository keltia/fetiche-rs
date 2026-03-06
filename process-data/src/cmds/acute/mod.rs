//! Module dealing with various ACUTE-specific data like our antennas, sites, etc.
//!
//! This provides a CRUD-like interface with subcommands like `add` & `delete`.
//!

use chrono::{DateTime, Utc};
use clap::Parser;
use eyre::Result;
use geo::coord;
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tabled::{settings::Style, Table, Tabled};
use tracing::trace;

use crate::runtime::Context;

pub(crate) use antennas::*;
pub(crate) use install::*;
pub(crate) use sites::*;

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
    #[clap(visible_alias = "a")]
    Antennas(AntennasOpts),
    /// Fetch which antenna was on a site and when.
    #[clap(visible_alias = "i", visible_alias = "inst")]
    Install(InstOpts),
    /// Display all sites.
    #[clap(visible_alias = "s")]
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

#[derive(Debug, Deserialize, Row, Serialize, Tabled)]
struct Antenna {
    pub id: i32,
    #[serde(rename = "type")]
    pub atype: String,
    pub name: String,
    pub owned: bool,
    pub description: String,
}

#[derive(Debug, Deserialize, Row, Serialize, Tabled)]
struct Install {
    pub install_id: i32,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    #[serde(rename = "type")]
    pub atype: String,
    pub antenna_name: String,
    pub site_name: String,
    pub site_id: i32,
    pub timezone: String,
}

#[derive(Debug, Deserialize, Row, Serialize, Tabled)]
struct Site {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub basename: String,
    pub latitude: f64,
    pub longitude: f64,
    pub ref_altitude: i32,
    pub timezone: String,
    pub offset_h: i32,
    pub distance_km: f64,
}

// ----- Dispatching

#[tracing::instrument(skip(ctx))]
pub async fn run_acute_cmd(ctx: &Context, opts: &AcuteOpts) -> Result<()> {
    trace!("run_acute_cmd");

    let dbh = ctx.db().await;
    match &opts.subcmd {
        // List all antennas
        //
        AcuteSubCommand::Antennas(opts) => {
            // Fetch antennas as Arrow
            //
            let res = dbh
                .query_collect::<Antenna>("SELECT * FROM antennas ORDER BY id ASC")
                .await?;

            println!("Listing all antennas:");

            let res = if opts.table {
                let mut table = Table::new(res.as_slice());
                table.with(Style::sharp());
                table.to_string()
            } else {
                json!(res).to_string()
            };
            println!("{res}");
        }
        // List all installations
        //
        AcuteSubCommand::Install(opts) => {
            // Find all installations with sites' name and antenna's ID
            //
            let r = r##"
SELECT * FROM deployments
ORDER BY start_at ASC
           "##;

            eprintln!("Listing all installations:");
            dbh.execute(r).await?;
            let q = QueryBuilder::new(r);
            let res = dbh.query_collect::<Install>(q).await?;

            let res = if opts.table {
                let mut table = Table::new(res.as_slice());
                table.with(Style::sharp());
                table.to_string()
            } else {
                json!(res).to_string()
            };
            println!("{res}");
        }
        AcuteSubCommand::Sites(opts) => {
            // This is our current location in Brétigny
            //
            let home = coord! {x: 48.600052, y:2.347038};

            match &opts.subcmd {
                SitesSubCommand::List => {
                    // Fetch sites
                    //
                    let r = r##"
SELECT
  id,
  name,
  code,
  basename,
  latitude,
  longitude,
  ref_altitude,
  timezone,
  offset AS offset_h,
  floor(dist_2d($1, $2, longitude, latitude) / 1000.) AS distance_km
FROM
  sites
ORDER BY
  id
    "##;
                    let q = QueryBuilder::new(r).arg(home.y).arg(home.x);
                    let res = dbh.query_collect::<Site>(q).await?;

                    println!("Listing all sites:");
                    let res = if opts.table {
                        let mut table = Table::new(res.as_slice());
                        table.with(Style::sharp());
                        table.to_string()
                    } else {
                        json!(res).to_string()
                    };

                    println!("{res}");
                }
                SitesSubCommand::Add(_opts) => todo!(),
                SitesSubCommand::Modify => todo!(),
                SitesSubCommand::Remove => todo!(),
            }
        }
    }

    Ok(())
}
