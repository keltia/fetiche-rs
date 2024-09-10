//! Export the distances calculated by the `distances` module.
//!

use clap::Parser;
use clickhouse::Client as CHClient;
use csv::WriterBuilder;
use eyre::Result;
use klickhouse::{ClientOptions, DateTime, Row};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{info, trace};

use crate::cmds::Format;
use crate::config::Context;

#[derive(Debug, Parser)]
pub struct ExpDistOpts {
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

/// Private struct for extracting data
///
#[derive(Debug, Deserialize, Row, Serialize)]
struct Encounter {
    site: String,
    en_id: String,
    time: DateTime,
    journey: i32,
    drone_id: String,
    model: String,
    drone_lon: f32,
    drone_lat: f32,
    drone_alt_m: f32,
    drone_height_m: f32,
    prox_callsign: String,
    prox_id: String,
    prox_lon: f32,
    prox_lat: f32,
    prox_alt_m: f32,
    distance_slant_m: i32,
    distance_hor_m: i32,
    distance_vert_m: i32,
    distance_home_m: i32,
}

//     en_id,
//     site,
//     time,
//     journey,
//     drone_id,
//     model,
//     drone_lat,
//     drone_lon,
//     drone_alt_m,
//     drone_height_m,
//     prox_callsign,
//     prox_id,
//     prox_lat,
//     prox_lon,
//     prox_alt_m,
//     distance_hor_m,
//     distance_vert_m,
//     distance_home_m,
//     distance_slant_m,

#[tracing::instrument(skip(_dbh))]
async fn export_all_encounters_records(_dbh: &CHClient) -> Result<Vec<Encounter>> {
    trace!("retrieving records from airplane_prox");

    let name = std::env::var("CLICKHOUSE_DB")?;
    let user = std::env::var("CLICKHOUSE_USER")?;
    let pass = std::env::var("CLICKHOUSE_PASSWD")?;
    let endpoint = std::env::var("KLICKHOUSE_URL")?;

    let client = klickhouse::Client::connect(
        endpoint,
        ClientOptions {
            username: user,
            password: pass,
            default_database: name,
        },
    )
        .await?;

    let r = r##"SELECT *
  FROM airplane_prox
  ORDER BY time
        "##;

    let res = client.query_collect::<Encounter>(r).await?;

    drop(client);

    Ok(res)
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
///
#[tracing::instrument(skip(dbh))]
async fn export_all_encounters_csv(dbh: &CHClient, fname: &str) -> Result<()> {
    trace!("Exporting all encounters from airplane_prox");

    let data = export_all_encounters_records(dbh).await?;
    let len = data.len();

    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new()
        .has_headers(true)
        .from_writer(vec![]);

    // Insert data
    //
    data.into_iter().for_each(|rec| {
        wtr.serialize(rec).unwrap();
    });

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    fs::write(fname, data)?;
    trace!("Exported {} encounters", len);

    Ok(())
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
/// Same as previous but export as a Parquet file.
///
#[tracing::instrument(skip(dbh))]
async fn export_all_encounters_parquet(dbh: &CHClient, fname: &str) -> eyre::Result<()> {
    eprintln!("Summary file");
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
  ORDER BY time
  INTO OUTFILE '{}'
  FORMAT parquet COMPRESSION 'zstd';
        "##,
        fname
    );

    dbh.query(&r).execute().await?;

    Ok(())
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
///
#[tracing::instrument(skip(dbh))]
async fn export_all_encounters_text(dbh: &CHClient) -> eyre::Result<()> {
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
  ORDER BY time
  FORMAT PrettyCompact
"##;

    dbh.query(r).execute().await?;

    Ok(())
}

#[tracing::instrument(skip(dbh))]
async fn export_all_encounters_summary_csv(dbh: &CHClient, fname: &str) -> eyre::Result<()> {
    // Create a temp file with all min distances
    //
    let r = r##"
CREATE OR REPLACE TABLE airprox_summary
ENGINE = Memory
AS (
  SELECT
    en_id,
    journey,
    drone_id,
    min(distance_slant_m) as distance_slant_m
  FROM
    airplane_prox
  GROUP BY
    en_id,journey,drone_id
)"##;

    dbh.query(r).execute().await?;

    // Match with airprox_summary for export
    //
    let r1 = format!(
        r##"
  SELECT
    a.en_id,
    a.site,
    a.time,
    a.journey,
    a.drone_id,
    a.model,
    a.drone_lat,
    a.drone_lon,
    a.drone_alt_m,
    a.drone_height_m,
    a.prox_callsign,
    a.prox_id,
    a.prox_lat,
    a.prox_lon,
    a.prox_alt_m,
    a.distance_hor_m,
    a.distance_vert_m,
    a.distance_home_m,
    a.distance_slant_m,
  FROM
    airplane_prox AS a JOIN airprox_summary AS s
    ON
    s.en_id = a.en_id AND
    s.journey = a.journey AND
    s.drone_id = a.drone_id
  WHERE
    a.distance_slant_m = s.distance_slant_m
  ORDER BY time
  INTO OUTFILE '{}' FORMAT CSVWithNames;
    "##,
        fname
    );

    dbh.query(&r1).execute().await?;

    Ok(())
}

#[tracing::instrument(skip(ctx))]
pub async fn export_results(ctx: &Context, opts: &ExpDistOpts) -> eyre::Result<()> {
    let dbh = ctx.db();

    // Do we export as a csv the "encounters of the day"?
    //
    match &opts.output {
        Some(fname) => {
            if opts.summary {
                export_all_encounters_summary_csv(&dbh, fname).await?
            } else {
                match opts.format {
                    Format::Csv => export_all_encounters_csv(&dbh, fname).await?,
                    Format::Parquet => export_all_encounters_parquet(&dbh, fname).await?,
                    _ => (),
                }
            };
        }
        None => {
            export_all_encounters_text(&dbh).await?;
        }
    }

    info!("Done.");
    Ok(())
}
