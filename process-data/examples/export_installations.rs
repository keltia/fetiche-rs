use std::env::var;
use std::fs::File;

use clickhouse::{Client, Row};
use eyre::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::fs;

const URL: &str = "http://127.0.0.1:8123";
const DB: &str = "acute";
const USER: &str = "default";
const PASS: &str = "";
const FNAME: &str = "installations.csv";

// CREATE TABLE acute.installations
// (
//     `id` Int32,
//     `site_id` Int32,
//     `antenna_id` Int32,
//     `start_at` DateTime,
//     `end_at` DateTime,
//     `comment` String
// )
// ENGINE = MergeTree
// PRIMARY KEY id
// ORDER BY id
// SETTINGS index_granularity = 8192
// COMMENT 'Which antenna on each site in time.'
//
#[derive(Debug, Deserialize, Serialize, Row)]
pub struct Install {
    pub id: u32,
    pub site_id: u32,
    pub antenna_id: u32,
    #[serde(with = "clickhouse::serde::time::datetime")]
    pub start_at: OffsetDateTime,
    #[serde(with = "clickhouse::serde::time::datetime")]
    pub end_at: OffsetDateTime,
    pub comment: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let url = var("CLICKHOUSE_URL").unwrap_or(URL.into());
    let pass = var("CLICKHOUSE_PASSWD").unwrap_or(PASS.into());

    let client = Client::default()
        .with_url(url)
        .with_user(USER)
        .with_password(pass)
        .with_database(DB)
        .with_option("wait_end_of_query", "1");

    let all = client
        .query("SELECT * FROM acute.installations")
        .fetch_all::<Install>().await?;

    let fh = File::create(FNAME)?;
    let mut wtr = csv::Writer::from_writer(fh);

    all.iter().for_each(|row| wtr.serialize(row).unwrap() );
    let _ = wtr.flush()?;

    // Check
    //
    if fs::try_exists(FNAME).await? {
        println!("Exported {} rows to {} ", all.len(), FNAME);
    }
    Ok(())
}

