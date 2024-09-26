//! Export the distances calculated by the `distances` module.
//!

use std::fmt::Debug;
use std::fs;

use clap::Parser;
use csv::WriterBuilder;
use datafusion::config::TableParquetOptions;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::{CsvReadOptions, SessionContext};
use eyre::Result;
use klickhouse::{Client, ClientOptions, DateTime, QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use tempfile::Builder;
use tracing::{debug, info, trace};

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
    site: i32,
    en_id: String,
    time: DateTime,
    journey: i32,
    drone_id: String,
    model: String,
    drone_lat: f32,
    drone_lon: f32,
    drone_alt_m: f32,
    drone_height_m: f32,
    prox_callsign: String,
    prox_id: String,
    prox_lat: f32,
    prox_lon: f32,
    prox_alt_m: f32,
    distance_slant_m: i32,
    distance_hor_m: i32,
    distance_vert_m: i32,
    distance_home_m: i32,
}

/// Retrieve all the records in `airplane_prox` table.
///
#[tracing::instrument(skip(client))]
async fn retrieve_all_encounters(client: &Client) -> Result<Vec<Encounter>> {
    trace!("retrieving records from airplane_prox");

    let r = r##"
  SELECT
    site,
    en_id,
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
    distance_slant_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m,
  FROM airplane_prox
  ORDER BY time
        "##;

    let res = client.query_collect::<Encounter>(r).await?;
    debug!("retrieved encounters: {:?}", res);

    Ok(res)
}

/// Retrieve the subset summary of all encounters from `airplane_prox`
///
#[tracing::instrument(skip(client))]
async fn retrieve_summary_encounters(client: &Client) -> Result<Vec<Encounter>> {
    trace!("retrieving summary records from airplane_prox");

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
    trace!("Create temp table airprox_summary");
    client.execute(r).await?;

    // Match with airprox_summary for export
    //
    let r1 = r##"
  SELECT *
  FROM
    airplane_prox AS a JOIN airprox_summary AS s
    ON
        s.en_id = a.en_id AND
        s.journey = a.journey AND
        s.drone_id = a.drone_id
  WHERE
    a.distance_slant_m = s.distance_slant_m
  ORDER BY time
    "##;
    let summ = client.query_collect::<Encounter>(r1).await?;
    trace!("Summary encounters: {:?}", summ);
    Ok(summ)
}

/// Write the output of `retrieve_all_encounters()` as a CSV file
///
#[tracing::instrument(skip(client))]
async fn export_all_encounters_csv(client: &Client, fname: &str) -> Result<()> {
    trace!("Exporting all encounters from airplane_prox");

    let data = retrieve_all_encounters(client).await?;
    let len = data.len();

    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new().has_headers(true).from_writer(vec![]);

    // Insert data
    //
    data.into_iter().for_each(|rec| {
        assert!(rec.distance_slant_m >= rec.distance_hor_m);
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
/// Same as previous but export as a Parquet file.  Due to the way DataFrames are handled in
/// Datafusion, it is easier to generate the CSV and use it to generate a parquet file.
///
#[tracing::instrument(skip(client))]
async fn export_all_encounters_parquet(client: &Client, fname: &str) -> Result<()> {
    let csv = Builder::new().suffix(".csv").tempfile()?;
    let tmpname = csv.path().to_string_lossy().to_string();
    trace!("Creating and saving CSV into {tmpname}");

    // Generate the csv file as `tmpname`
    //
    export_all_encounters_csv(client, &tmpname).await?;

    let ctx = SessionContext::new();
    let df = ctx
        .read_csv(&tmpname, CsvReadOptions::default().has_header(true))
        .await?;
    let dfopts = DataFrameWriteOptions::default().with_single_file_output(true);

    let mut options = TableParquetOptions::default();
    options.global.created_by = "process-data/export".to_string();
    options.global.writer_version = "2.0".to_string();
    options.global.encoding = Some("plain".to_string());
    options.global.statistics_enabled = Some("page".to_string());
    options.global.compression = Some("zstd(8)".to_string());

    trace!("Writing {fname} as parquet.");
    let _ = df.write_parquet(fname, dfopts, Some(options)).await?;

    eprintln!("Summary file {fname}");

    Ok(())
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
///
#[tracing::instrument(skip(client))]
async fn export_all_encounters_text(client: &Client) -> Result<()> {
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
    distance_slant_m,
    distance_hor_m,
    distance_vert_m,
    distance_home_m,
  FROM airplane_prox
  ORDER BY time
  FORMAT PrettyCompact
"##;
    let q = QueryBuilder::new(r);
    let _ = client.query_collect::<Encounter>(q).await?;

    Ok(())
}

#[tracing::instrument(skip(dbh))]
async fn export_all_encounters_summary_csv(dbh: &Client, fname: &str) -> eyre::Result<()> {
    // Create a temp file with all min distances
    //
    let data = retrieve_summary_encounters(dbh).await?;
    let len = data.len();

    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new().has_headers(true).from_writer(vec![]);

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

/// Main entry point for the various `export distances` subcommand.
///
#[tracing::instrument(skip(_ctx))]
pub async fn export_results(ctx: &Context, opts: &ExpDistOpts) -> eyre::Result<()> {
    let client = ctx.db().await;

    // Do we export as a csv the "encounters of the day"?
    //
    match &opts.output {
        Some(fname) => {
            if opts.summary {
                export_all_encounters_summary_csv(&client, fname).await?
            } else {
                match opts.format {
                    Format::Csv => export_all_encounters_csv(&client, fname).await?,
                    Format::Parquet => export_all_encounters_parquet(&client, fname).await?,
                    _ => (),
                }
            };
        }
        None => {
            export_all_encounters_text(&client).await?;
        }
    }
    drop(client);
    info!("Done.");
    Ok(())
}
