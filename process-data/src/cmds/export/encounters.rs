//! Module for exporting encounters into KML files.
//!

use std::path::PathBuf;

use crate::cmds::Format;
use crate::config::Context;
use clap::Parser;
use eyre::{format_err, Result};
use klickhouse::{QueryBuilder, Row};

#[derive(Debug, Parser)]
pub struct ExpEncountOpts {
    /// Export every encounter in its own file.
    #[clap(short = 'A', long)]
    all: bool,
    /// Export that Encounter ID
    #[clap(long)]
    id: Option<String>,
    /// Format (default is KML)
    #[clap(short = 'F', long, default_value = "kml", value_parser)]
    format: Format,
    /// Output file or directory.
    #[clap(short = 'o', long)]
    output: Option<PathBuf>,
}

pub async fn export_encounters(ctx: &Context, opts: &ExpEncountOpts) -> Result<()> {
    let client = ctx.db().await;

    // Check arguments
    //
    if opts.all && opts.id.is_some() {
        return Err(format_err!("Either -A or --id, not both!"));
    }

    if opts.all && !opts.output.is_dir() {
        return Err(format_err!("output path {} given, expected a directory", opts.output));
    }

    Ok(())
}

async fn export_one_encounter(ctx: &Context, id: &str) -> Result<String> {
    let client = ctx.db().await;

    #[derive(Clone, Debug, Row)]
    struct PlanePoint {
        site: u32,
        journey: u32,
        prox_callsign: String,
        prox_id: String,
        drone_id: String,
    }

    // Fetch the drone & airplane IDs
    //
    let rp = r##"
SELECT
  site, journey, prox_callsign, prox_id, drone_id
FROM airprox_summary
WHERE id = $1
    "##;
    let q = QueryBuilder::new(rp).arg(id);
    let res = client.query_one::<PlanePoint>(q).await?;

    let drone_id = res.drone_id.clone();
    let prox_id = res.prox_id.clone();

    // Fetch plane points
    //
    let rpp = r##""##;

    // Fetch drone points
    //
    let rdp = r##""##;

    Ok(res)
}

async fn export_all_encounter(ctx: &Context) -> Result<String> {}

