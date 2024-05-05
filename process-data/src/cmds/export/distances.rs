//! Export the distances calculated by the `distances` module.
//!

use chrono::{Datelike, DateTime, TimeZone, Utc};
use clap::Parser;
use clickhouse::Client;
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
async fn export_all_encounters_csv(
    dbh: &Client,
    name: &str,
    day: DateTime<Utc>,
    fname: &str,
) -> eyre::Result<usize> {
    let r = format!(
        r##"
  SELECT
    en_id,
    site,
    time,
    journey,
    drone_id,
    model,
    drone_lat,
    drone_lon,
    drone_alt_m,
    drone_height_m,
    prox_callsign,
    prox_id,
    prox_lat,
    prox_lon,
    prox_alt_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m,
    distance_slant_m,
  FROM airplane_prox
  WHERE
    site = ? AND
    CAST(time AS DATE) >= CAST(? AS DATE) AND
    CAST(time AS DATE) < date_add(CAST(? AS DATE), INTERVAL 1 DAY)
    ORDER BY time
  INTO OUTFILE '{}' FORMAT parquet COMPRESSION zstd
        "##,
        fname
    );

    let count = dbh.query(&r)
        .bind(name)
        .bind(day)
        .bind(day)
        .fetch::<usize>()
        .await?;

    Ok(count)
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
/// Same as previous but export as a Parquet file.
///
#[tracing::instrument(skip(dbh))]
fn export_all_encounters_parquet(
    dbh: &Client,
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
    drone_lat,
    drone_lon,
    drone_alt_m,
    drone_height_m,
    prox_callsign,
    prox_id,
    prox_lat,
    prox_lon,
    prox_alt_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m,
    distance_slant_m,
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
fn export_all_encounters_text(dbh: &Client, name: &str, day: DateTime<Utc>) -> eyre::Result<usize> {
    let r = r##"
  SELECT
    en_id,
    site,
    time,
    journey,
    drone_id,
    model,
    drone_lat,
    drone_lon,
    drone_alt_m,
    drone_height_m,
    prox_callsign,
    prox_id,
    prox_lat,
    prox_lon,
    prox_alt_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m,
    distance_slant_m,
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
fn export_all_encounters_summary_csv(dbh: &Client, name: &str, day: DateTime<Utc>, fname: &str) -> eyre::Result<usize> {
    let r = format!(r##"
COPY (
  SELECT
    en_id,
    any_value(site) AS site,
    any_value(time) AS time,
    journey,
    drone_id,
    any_value(model) AS model,
    any_value(drone_lat) AS drone_lat,
    any_value(drone_lon) AS drone_lon,
    any_value(drone_alt_m) AS drone_alt_m,
    any_value(drone_height_m) AS drone_height_m,
    any_value(prox_callsign) AS prox_callsign,
    any_value(prox_id) AS prox_id,
    any_value(prox_lat) AS prox_lat,
    any_value(prox_lon) AS prox_lon,
    any_value(prox_alt_m) AS prox_alt_m,
    any_value(distance_hor_m) AS distance_hor_m,
    any_value(distance_vert_m) AS distance_vert_m,
    any_value(distance_home_m) as distance_home_m,
    MIN(distance_slant_m) AS distance_slant_m,
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
