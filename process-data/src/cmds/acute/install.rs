use crate::cmds::{CrudSubCommand, DBVars};
use crate::make_query;
use crate::runtime::Context;

use chrono::{DateTime, Utc};
use clap::Parser;
use eyre::Result;
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tabled::settings::Style;
use tabled::{Table, Tabled};

/// "acute installations"
///
#[derive(Debug, Parser)]
pub(crate) struct InstOpts {
    #[clap(short = 'C', long)]
    pub csv: bool,
    #[clap(short = 'J', long)]
    pub json: bool,
    #[clap(short = 'T', long, default_value = "true")]
    pub table: bool,
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
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

pub(crate) async fn install_list(ctx: &Context, opts: &InstOpts) -> Result<()> {
    // Prepare DB environment.
    //
    let planedb = ctx.config["planedb"].clone();
    let dronedb = ctx.config["dronedb"].clone();
    let workdb = ctx.config["workdb"].clone();

    let dbvars = DBVars {
        planedb,
        dronedb,
        workdb,
        tag: "".into(),
    };
    let dbh = ctx.db().await;

    // Find all installations with sites' name and antenna's ID
    //
    let r = make_query!(
        r##"
SELECT * FROM {workdb}.deployments
ORDER BY start_at ASC
"##,
        dbvars
    );

    eprintln!("Listing all installations:");

    let q = QueryBuilder::new(&r);
    let res = dbh.query_collect::<Install>(q).await?;

    let res = if opts.table {
        let mut table = Table::new(res.as_slice());
        table.with(Style::sharp());
        table.to_string()
    } else {
        json!(res).to_string()
    };
    eprintln!("{res}");
    Ok(())
}
