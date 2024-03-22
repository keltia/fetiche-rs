//! Export the distances calculated by the `distances` module.
//!

use chrono::{Datelike, DateTime, TimeZone, Utc};
use clap::Parser;
use duckdb::{Connection, params};
use duckdb::arrow::util::pretty::print_batches;
use tracing::info;

use crate::cmds::Format;
use crate::config::Context;

#[derive(Debug, Parser)]
pub struct ExpDistOpts {
    /// Export results for this site
    pub name: String,
    /// Day to export
    pub date: String,
    /// Summary or everything?
    #[clap(short = 'S', long)]
    pub summary: bool,
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
fn export_all_encounters_csv(
    dbh: &Connection,
    name: &str,
    day: DateTime<Utc>,
    fname: &str,
) -> eyre::Result<usize> {
    let r = format!(
        r##"
COPY (
  SELECT
    en_id,
    site,
    time,
    journey,
    drone_id,
    model,
    dy AS drone_lat,
    dx AS drone_lon,
    dz AS drone_alt,
    dh AS drone_height,
    callsign
    addr,
    py AS plane_lat,
    px AS plane_lon,
    distancelat AS distance_lat,
    distancevert AS distance_vert,
    distancehome as distance_home,
    distance,
  FROM airplane_prox
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
/// Same as previous but export as a Parquet file.
///
#[tracing::instrument(skip(dbh))]
fn export_all_encounters_parquet(
    dbh: &Connection,
    name: &str,
    day: DateTime<Utc>,
    fname: &str,
) -> eyre::Result<usize> {
    eprintln!("Summary file");
    let r = format!(
        r##"
COPY (
  SELECT
    en_id,
    site,
    time,
    journey,
    drone_id,
    model,
    dy AS drone_lat,
    dx AS drone_lon,
    dz AS drone_alt,
    dh AS drone_height,
    callsign
    addr,
    py AS plane_lat,
    px AS plane_lon,
    distancelat AS distance_lat,
    distancevert AS distance_vert,
    distancehome as distance_home,
    distance,
  FROM airplane_prox
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

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
///
#[tracing::instrument(skip(dbh))]
fn export_all_encounters_text(dbh: &Connection, name: &str, day: DateTime<Utc>) -> eyre::Result<usize> {
    let r = r##"
  SELECT
    en_id,
    site,
    time,
    journey,
    drone_id,
    model,
    dy AS drone_lat,
    dx AS drone_lon,
    dz AS drone_alt,
    dh AS drone_height,
    callsign
    addr,
    py AS plane_lat,
    px AS plane_lon,
    distancelat AS distance_lat,
    distancevert AS distance_vert,
    distancehome as distance_home,
    distance,
  FROM airplane_prox
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

#[tracing::instrument(skip(dbh))]
fn export_all_encounters_summary_csv(dbh: &Connection, name: &str, day: DateTime<Utc>, fname: &str) -> eyre::Result<usize> {
    let r = format!(r##"
COPY (
  SELECT
    en_id,
    any_value(site) AS site,
    any_value(time) AS time,
    journey,
    drone_id,
    any_value(model) AS model,
    any_value(dy) AS drone_lat,
    any_value(dx) AS drone_lon,
    any_value(dz) AS drone_alt,
    any_value(dh) AS drone_height,
    any_value(callsign) AS callsign,
    any_value(addr) AS addr,
    any_value(py) AS plane_lat,
    any_value(px) AS plane_lon,
    any_value(distancelat) AS distance_lat,
    any_value(distancevert) AS distance_vert,
    any_value(distancehome) as distance_home,
    MIN(distance) AS distance,
  FROM airplane_prox
  WHERE
    site = ? AND
    CAST(time AS DATE) >= CAST(? AS DATE) AND
    CAST(time AS DATE) < date_add(CAST(? AS DATE), INTERVAL 1 DAY)
  GROUP BY ALL
  ORDER BY time
) TO '{}' WITH (FORMAT CSV, HEADER true, DELIMITER ',');
    "##, fname);

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
            let count = if opts.summary {
                export_all_encounters_summary_csv(&dbh, &name, day, fname)?
            } else {
                match opts.format {
                    Format::Csv => export_all_encounters_csv(&dbh, &name, day, fname)?,
                    Format::Parquet => export_all_encounters_parquet(&dbh, &name, day, fname)?,
                    _ => 0,
                }
            };
            println!("Exported {} records to {}", count, fname);
        }
        None => {
            let _ = export_all_encounters_text(&dbh, &name, day)?;
        }
    }

    info!("Done.");
    Ok(())
}
