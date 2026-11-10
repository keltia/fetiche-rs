use crate::cmds::{CrudSubCommand, DBVars, Format, render_output};
use crate::make_query;
use crate::runtime::Context;

use clap::Parser;
use eyre::Result;
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

/// "acute antennas"
///
#[derive(Debug, Parser)]
pub struct AntennasOpts {
    /// Output format
    #[clap(short = 'F', long, default_value = "table")]
    pub format: Format,
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

pub async fn antennas_list(ctx: &Context, opts: &AntennasOpts) -> Result<()> {
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

    let res = render_output(&res, opts.format)?;
    eprintln!("{res}");
    Ok(())
}
