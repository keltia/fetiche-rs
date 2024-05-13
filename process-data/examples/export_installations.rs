use clickhouse::Client;
use eyre::Result;
use tokio::fs;

const URL: &str = "http://127.0.0.1:8123";
const DB: &str = "acute";
const USER: &str = "default";
const FNAME: &str = "/tmp/installations.csv";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let client = Client::default()
        .with_url(URL)
        .with_user(USER)
        .with_database(DB)
        .with_option("wait_end_of_query", "1");

    let val = client
        .query("SELECT geoDistance(2.319671,48.573174,2.303015, 48.566757) AS dist")
        .fetch_one::<f64>()
        .await?;

    eprintln!("val={val}");

    let r = r##"
  SELECT * FROM acute.installations INTO OUTFILE ? TRUNCATE AND STDOUT FORMAT CSV
"##;

    let _ = client
        .query(r)
        .bind(FNAME)
        .execute()
        .await?;

    // Check
    //
    if fs::try_exists(FNAME).await? {
        println!("Exported  rows to {} ", FNAME);
    }
    Ok(())
}

