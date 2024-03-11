use chrono::{DateTime, Utc};
use eyre::Result;

async fn test_humantime() {
    let base = "2024-03-08 12:34:56";

    let curr = humantime::parse_rfc3339_weak(base).unwrap();
    let curr: DateTime<Utc> = curr.into();
    dbg!(curr);
}

#[tokio::main]
async fn main() -> Result<()> {
    test_humantime().await;

    Ok(())
}

