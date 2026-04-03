//! This is the Rust equivalent of [import-adsb.py] with batching capabilities
//!

use crate::cmds::DBVars;
use crate::make_query;
use crate::runtime::Context;
use std::path::Path;

use clap::Parser;
use eyre::{eyre, Result};
use klickhouse::{QueryBuilder, Row};
use polars::prelude::{CsvReadOptions, NamedFrom, SerReader, Series};
use regex::Regex;
use serde::Deserialize;
use tracing::{debug, trace};

/// `import adsb` options
///
#[derive(Debug, Parser)]
pub struct AdsbOpts {
    /// Table name
    #[clap(short = 'T', long)]
    pub table: String,
    /// Batch import by this number of lines
    #[clap(short = 't', long, default_value = "500000")]
    pub threshold: usize,
    /// Filename
    pub fname: String,
}

#[derive(Debug, Deserialize, Row)]
struct Site {
    id: u32,
    name: String,
}

/// Import a single large CSV file into a given table in Clickhouse.
///
#[tracing::instrument(skip(ctx))]
pub async fn import_adsb(ctx: &Context, opts: &AdsbOpts) -> Result<()> {
    // Get the table name (possibly prefixed by namespace/database)
    //
    let table = opts.table.clone();
    let fname = opts.fname.clone();

    let sname = Path::new(fname.as_str())
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    debug!("sname={}", sname);

    // Filename should be formatted like this
    // `<basename>_YYYY-MM-DD`
    //
    let re = Regex::new(
        r##"^(?<basename>[[:alnum:]])_(?<year>[[:digit:]]{4})-(?<month>[[:digit:]]{2})-(?<day>[[:digit:]]{2})$"##,
    )?;
    let (basename, year, month, day) = if let Some(caps) = re.captures(sname) {
        (
            &caps[1].to_string(),
            &caps[2].parse::<u32>()?,
            &caps[3].parse::<u32>()?,
            &caps[4].parse::<u32>()?,
        )
    } else {
        return Err(eyre!("Bad filename {fname}"));
    };
    trace!("handling {basename} from={year}-{month}-{day}");

    // Retrieve site ID
    //
    let site = fetch_site_id(ctx, &basename).await?;
    debug!("site_name={} site_id={}", site.name, site.id);

    let mut df = CsvReadOptions::default()
        .with_chunk_size(opts.threshold)
        .try_into_reader_with_file_path(Some(fname.into()))?
        .finish()?;

    let site_col = Series::new("site".into(), vec![site.id; df.height()]);
    df.insert_column(0, site_col.into())?;

    debug!("df schema is {:?}", df.schema());

    Ok(())
}

/// Fetch the site ID from the databases with the specified basename
///
#[tracing::instrument(skip(ctx))]
async fn fetch_site_id(ctx: &Context, name: &str) -> Result<Site> {
    let db = ctx.db().await;
    let dbvars = DBVars::from_ctx(&ctx);

    // Fetch all sites long names and id
    //
    let r = make_query!(
        r##"
    SELECT id, name
    FROM {workdb}.sites
    WHERE basename = $1
    "##,
        dbvars
    );
    let q = QueryBuilder::new(&r).arg(name);
    let site = db.query_one::<Site>(q).await?;
    debug!("basename={name} id={} name={}", site.id, site.name);

    Ok(site)
}
