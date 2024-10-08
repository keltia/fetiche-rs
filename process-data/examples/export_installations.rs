use std::env::var;
use std::fs::File;

use eyre::Result;
use klickhouse::{Client, ClientOptions, DateTime, Row};
use serde::{Deserialize, Serialize};
use tokio::fs;

const URL: &str = "http://127.0.0.1:9000";
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
    pub start_at: DateTime,
    pub end_at: DateTime,
    pub comment: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let url = var("KLICKHOUSE_URL").unwrap_or(URL.into());
    let name = var("CLICKHOUSE_DB").unwrap_or(DB.into());
    let user = var("CLICKHOUSE_USER").unwrap_or(USER.into());
    let pass = var("CLICKHOUSE_PASSWD").unwrap_or(PASS.into());

    let client = Client::connect(
        url.clone(),
        ClientOptions {
            username: user.clone(),
            password: pass.clone(),
            default_database: name.clone(),
            ..Default::default()
        },
    )
    .await?;

    let all = client
        .query_collect::<Install>("SELECT * FROM acute.installations")
        .await?;

    let fh = File::create(FNAME)?;
    let mut wtr = csv::Writer::from_writer(fh);

    all.iter().for_each(|row| wtr.serialize(row).unwrap());
    let _ = wtr.flush()?;

    // Check
    //
    if fs::try_exists(FNAME).await? {
        println!("Exported {} rows to {} ", all.len(), FNAME);
    }
    Ok(())
}
