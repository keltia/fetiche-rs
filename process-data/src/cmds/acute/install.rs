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

use fetiche_common::Delim;

/// "acute installations"
///
#[derive(Debug, Parser)]
pub struct InstOpts {
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

pub async fn install_list(ctx: &Context, opts: &InstOpts) -> Result<()> {
    // Prepare DB environment.
    //
    let dbvars = DBVars::from_ctx(ctx);
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

    let res = if opts.json {
        json!(&res).to_string()
    } else if opts.csv {
        Delim::Comma.prepare_csv(&res, true)?
    } else {
        let mut table = Table::new(res.as_slice());
        table.with(Style::sharp());
        table.to_string()
    };
    eprintln!("{res}");
    Ok(())
}
