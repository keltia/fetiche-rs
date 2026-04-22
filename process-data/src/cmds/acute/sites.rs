use clap::Parser;
use eyre::Result;
use geo::coord;
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tabled::settings::Style;
use tabled::{Table, Tabled};

use fetiche_common::Delim;

use crate::cmds::DBVars;
use crate::make_query;
use crate::runtime::Context;

/// "acute sites"
///
#[derive(Debug, Parser)]
pub struct SiteOpts {
    #[clap(short = 'c', long)]
    pub csv: bool,
    #[clap(short = 'T', long, default_value = "true")]
    pub table: bool,
    #[clap(subcommand)]
    pub subcmd: SitesSubCommand,
}

#[derive(Debug, Parser)]
pub enum SitesSubCommand {
    /// Add something
    Add(AddSiteOpts),
    /// Modify something
    Modify,
    /// Remove something
    Remove,
    /// Default is listing everything
    List(SitesListOpts),
}

#[derive(Debug, Parser)]
pub struct SitesListOpts {
    #[clap(short = 'C', long)]
    pub csv: bool,
    #[clap(short = 'J', long)]
    pub json: bool,
    #[clap(short = 'T', long)]
    pub table: bool,
}

#[derive(Debug, Parser)]
pub struct FindOpts {
    pub name: String,
}

#[derive(Debug, Default, Parser)]
pub struct AddSiteOpts {
    name: String,
    lat: f64,
    lon: f64,
    alt: i32,
    basename: String,
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

#[allow(dead_code)]
fn insert_new_site(_opts: &AddSiteOpts) -> Result<()> {
    Ok(())
}

///  acute site subcommand handling.
///
#[tracing::instrument(skip(ctx))]
pub async fn sites_list(ctx: &Context, opts: &SitesListOpts) -> Result<()> {
    // Brétigny for you.
    //
    let home = coord! {x: 48.600052, y:2.347038};

    // Prepare DB environment.
    //
    let dbvars = DBVars::from_ctx(ctx);
    let dbh = ctx.db().await;

    let r = make_query!(
        r##"
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
  {workdb}.sites
ORDER BY
  id
    "##,
        dbvars
    );
    let q = QueryBuilder::new(&r).arg(home.y).arg(home.x);
    let res = dbh.query_collect::<Site>(q).await?;

    println!("Listing all sites:");
    let res = if opts.json {
        json!(&res).to_string()
    } else if opts.csv {
        Delim::Comma.prepare_csv(&res, true)?
    } else {
        let mut table = Table::new(res.as_slice());
        table.with(Style::sharp());
        table.to_string()
    };

    println!("{res}");
    Ok(())
}
