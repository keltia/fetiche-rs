//! This is the Rust equivalent of [import-adsb.py] with batching capabilities
//!

use crate::config::Context;
use clap::Parser;
use klickhouse::Row;
use datafusion::prelude::{CsvReadOptions, SessionConfig, SessionContext};
use serde::Deserialize;
use tracing::debug;

/// `import adsb` options
///
#[derive(Debug, Parser)]
pub struct AdsbOpts {
    /// Table name
    #[clap(short = 'T', long)]
    pub table: String,
    /// Batch import by this number of lines
    #[clap(short = 't', long, default_value = "100_000")]
    pub threshold: u32,
    /// Filename
    pub fname: String,
}

#[derive(Debug, Deserialize, Row)]
struct Sites {
    name: String,
    id: u32,
}

/// Fetch all sites from the databases with id and long name
///
#[tracing::instrument(skip(ctx))]
pub async fn fetch_sites(ctx: &Context) -> eyre::Result<Vec<Sites>> {
    let db = ctx.db().await;

    // Fetch all sites long names and id
    //
    let r = r##"
    SELECT name, id FROM sites
    "##;
    let sites = db.query(r).fetch_all::<Sites>().await?;
    Ok(sites)
}

/// Import a single large CSV file into a given table in Clickhouse.
///
#[tracing::instrument(skip(ctx))]
pub async fn import_adsb(ctx: &Context, opts: &AdsbOpts) -> eyre::Result<()> {
    let db = ctx.db().await;

    let sites = fetch_sites(ctx).await?;
    debug!("sites={:?}", sites);

    let table = &opts.table;
    let fname = &opts.fname;

    // Setup datafusion for csv file in batches
    //
    let cfg_opts = SessionConfig::new()
        .with_batch_size(opts.threshold as usize);
    let ctx = SessionContext::new_with_config(cfg_opts);
    let csv_opts = CsvReadOptions::new().has_header(true);

    // Do the reading
    //
    ctx.register_csv("import-adsb", fname, csv_opts).await?;
    let df = ctx.sql("SELECT * FROM import-adsb").await?;

    let mut rows = df.execute_stream_partitioned().await?;
    rows.iter().for_each(|row| {});

    Ok(())
}
