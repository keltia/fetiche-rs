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

use fetiche_common::Delim;

/// "acute antennas"
///
#[derive(Debug, Parser)]
pub(crate) struct AntennasOpts {
    #[clap(short = 'C', long)]
    pub csv: bool,
    #[clap(short = 'J', long)]
    pub json: bool,
    #[clap(short = 'T', long)]
    pub table: bool,
    #[clap(subcommand)]
    pub subcmd: Option<CrudSubCommand>,
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
    let dbvars = DBVars::from_ctx(ctx);
    let dbh = ctx.db().await;

    // Fetch antennas as Arrow
    //
    let r = make_query!("SELECT * FROM {workdb}.antennas ORDER BY id ASC", dbvars);
    let q = QueryBuilder::new(&r);
    let res = dbh.query_collect::<Antenna>(q).await?;

    eprintln!("Listing all antennas:");

    let res = if opts.json {
        json!(&res).to_string()
    } else if opts.csv {
        Delim::Colon.prepare_csv(&res, true)?
    } else {
        let mut table = Table::new(res.as_slice());
        table.with(Style::sharp());
        table.to_string()
    };
    eprintln!("{res}");
    Ok(())
}
