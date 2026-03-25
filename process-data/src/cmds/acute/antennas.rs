use crate::cmds::{CrudSubCommand, DBVars};
use crate::make_query;
use crate::runtime::Context;

use clap::Parser;
use eyre::Result;
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tabled::settings::Style;
use tabled::{Table, Tabled};

/// "acute antennas"
///
#[derive(Debug, Parser)]
pub(crate) struct AntennasOpts {
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
    #[clap(short = 'c', long)]
    pub csv: bool,
    #[clap(short = 'T', long, default_value = "true")]
    pub table: bool,
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

pub(crate) async fn antennas_list(ctx: &Context, opts: &AntennasOpts) -> Result<()> {
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

    // Fetch antennas as Arrow
    //
    let r = make_query!("SELECT * FROM {workdb}.antennas ORDER BY id ASC", dbvars);
    let q = QueryBuilder::new(&r);
    let res = dbh.query_collect::<Antenna>(q).await?;

    eprintln!("Listing all antennas:");

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
