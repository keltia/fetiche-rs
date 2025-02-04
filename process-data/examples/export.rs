//! Example: Export data from ClickHouse to a CSV file.
//!
//! This example demonstrates how to connect to a ClickHouse database,
//! query data, and export the results as a CSV file. The example uses
//! the `klickhouse` library for database interaction and the `csv` library
//! for generating the CSV output.
//!
//! Usage:
//! - Set environment variables:
//!   - `CLICKHOUSE_DB`: Name of the ClickHouse database.
//!   - `CLICKHOUSE_USER`: Username for ClickHouse.
//!   - `CLICKHOUSE_PASSWD`: Password for ClickHouse.
//!   - `KLICKHOUSE_URL`: Base URL for the ClickHouse endpoint.
//! - Use the `--fname` option to specify the output file name (default: `all_encounters.csv`).
//!
//! The exported data includes information about drone encounters, such as
//! drone and proximate object details, locations, distances, and identifiers.
//!

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use csv::WriterBuilder;
use eyre::Result;
use klickhouse::{Client, ClientOptions, DateTime, QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use tokio::fs;

use fetiche_common::{close_logging, init_logging};

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

#[derive(Debug, Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
struct Opts {
    #[clap(short = 'o', long, default_value = "all_encounters.csv")]
    fname: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let name = std::env::var("CLICKHOUSE_DB")?;
    let user = std::env::var("CLICKHOUSE_USER")?;
    let pass = std::env::var("CLICKHOUSE_PASSWD")?;
    let endpoint = std::env::var("KLICKHOUSE_URL")?;

    init_logging("export-encounters", false, true, None)?;

    eprintln!("Create connection.");
    let client = Client::connect(
        endpoint,
        ClientOptions {
            username: user,
            password: pass,
            default_database: name,
            ..Default::default()
        },
    )
        .await?;

    let fname = opts.fname.clone();
    eprintln!("Created fname: {}", fname);
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
        "##;

    let q = QueryBuilder::new(&r);
    let res = client.query_collect::<Encounter>(q).await?;

    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new().has_headers(true).from_writer(vec![]);

    // Insert data
    //
    res.iter().for_each(|rec| {
        wtr.serialize(rec).unwrap();
    });

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    fs::write(fname, data).await?;

    close_logging();
    Ok(())
}
