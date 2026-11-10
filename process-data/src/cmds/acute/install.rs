use crate::cmds::{CrudSubCommand, DBVars, Format, render_output};
use crate::make_query;
use crate::runtime::Context;

use chrono::{DateTime, Utc};
use clap::Parser;
use eyre::Result;
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

/// "acute installations"
///
#[derive(Debug, Parser)]
pub struct InstOpts {
    /// Output format
    #[clap(short = 'F', long, default_value = "table")]
    pub format: Format,
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

    let res = render_output(&res, opts.format)?;
    eprintln!("{res}");
    Ok(())
}
