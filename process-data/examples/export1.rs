//! ClickHouse official client `clickhouse`.
//!
//! NOTE: current official client can not handle `DateTime<Utc>` at all and we need to use
//! `OffsetDateTime` will give us a UNIX timestamp.
//!

use std::fs;

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use clickhouse::{Client, Row};
use csv::WriterBuilder;
use eyre::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use fetiche_common::{close_logging, init_logging};

#[derive(Debug, Deserialize, Row, Serialize)]
struct Encounter {
    site: String,
    en_id: String,
    #[serde(with = "clickhouse::serde::time::datetime")]
    time: OffsetDateTime,
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
    let endpoint = std::env::var("CLICKHOUSE_URL")?;

    init_logging("export-encounters", false, true, true)?;

    eprintln!("Connecting to {} @ {}", name, endpoint);
    let dbh = Client::default()
        .with_url(endpoint.clone())
        .with_database(&name)
        .with_user(&user)
        .with_password(pass);

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

    let res = dbh.query(&r).fetch_all::<Encounter>().await?;

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
    fs::write(fname, data)?;

    close_logging();
    Ok(())
}
