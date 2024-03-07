//! Export the distances calculated by the `distances` module.
//!

use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use duckdb::arrow::util::pretty::print_batches;
use duckdb::{params, Connection};
use tracing::info;

use crate::cmds::Format;
use crate::config::Context;

#[derive(Debug, Parser)]
pub struct ExpDistOpts {
    /// Export results for this site
    pub name: String,
    /// Day to export
    pub date: String,
    /// Output format
    #[clap(short = 'F', long, default_value = "csv")]
    pub format: Format,
    /// Output file
    #[clap(short = 'o', long)]
    pub output: Option<String>,
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
///
#[tracing::instrument(skip(dbh))]
fn export_distances(
    dbh: &Connection,
    name: &str,
    day: DateTime<Utc>,
    fname: &str,
) -> eyre::Result<usize> {
    let r = format!(
        r##"
COPY (
  SELECT * FROM encounters
  WHERE
    site = ? AND
    CAST(time AS DATE) >= CAST(? AS DATE) AND
    CAST(time AS DATE) < date_add(CAST(? AS DATE), INTERVAL 1 DAY)
    ORDER BY time
) TO '{}' WITH (FORMAT CSV, HEADER true, DELIMITER ',');
        "##,
        fname
    );

    let mut stmt = dbh.prepare(&r)?;
    let count = stmt.execute(params![name, day, day])?;

    Ok(count)
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
///
#[tracing::instrument(skip(dbh))]
fn export_distances_text(dbh: &Connection, name: &str, day: DateTime<Utc>) -> eyre::Result<usize> {
    let r = r##"
  SELECT * FROM encounters
  WHERE
    site = ? AND
    CAST(time AS DATE) >= CAST(? AS DATE) AND
    CAST(time AS DATE) < date_add(CAST(? AS DATE), INTERVAL 1 DAY)
    ORDER BY time
"##;
    let mut stmt = dbh.prepare(r)?;
    let rbs: Vec<_> = stmt.query_arrow(params![name, day, day])?.collect();
    print_batches(&rbs)?;

    Ok(rbs.len())
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
/// Same as previous but export as a Parquet file.
///
#[tracing::instrument(skip(dbh))]
fn export_distances_parquet(
    dbh: &Connection,
    name: &str,
    day: DateTime<Utc>,
    fname: &str,
) -> eyre::Result<usize> {
    let r = format!(
        r##"
COPY (
  SELECT * FROM encounters
  WHERE
    site = ? AND
    CAST(time AS DATE) >= CAST(? AS DATE) AND
    CAST(time AS DATE) < date_add(CAST(? AS DATE), INTERVAL 1 DAY)
    ORDER BY time
) TO '{}' WITH (FORMAT 'parquet', COMPRESSION 'zstd', ROW_GROUP_SIZE 1048576);
        "##,
        fname
    );

    let mut stmt = dbh.prepare(&r)?;
    let count = stmt.execute(params![name, day, day])?;

    Ok(count)
}

#[tracing::instrument(skip(ctx))]
pub fn export_results(ctx: &Context, opts: &ExpDistOpts) -> eyre::Result<()> {
    let dbh = ctx.db();

    let tm = dateparser::parse(&opts.date).unwrap();
    let day = Utc
        .with_ymd_and_hms(tm.year(), tm.month(), tm.day(), 0, 0, 0)
        .unwrap();
    info!("Exporting results for {}:", day);

    // Load parameters
    //
    let name = opts.name.clone();

    // Do we export as a csv the "encounters of the day"?
    //
    match &opts.output {
        Some(fname) => {
            let count = match opts.format {
                Format::Csv => export_distances(&dbh, &name, day, fname)?,
                Format::Parquet => export_distances_parquet(&dbh, &name, day, fname)?,
                _ => 0,
            };
            println!("Exported {} records to {}", count, fname);
        }
        None => {
            let _ = export_distances_text(&dbh, &name, day)?;
        }
    }

    info!("Done.");
    Ok(())
}
