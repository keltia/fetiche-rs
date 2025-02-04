use chrono::{DateTime, Utc};
use eyre::Result;
use humantime::parse_duration;
use jiff::Timestamp;

async fn test_humantime() -> Result<()> {
    let base = "2024-03-08 00:00:00";

    let curr = humantime::parse_rfc3339_weak(base)?;
    eprintln!("1={curr:?}");

    let added = parse_duration("3h 5min")?;
    let curr = curr + added;
    eprintln!("2={curr:?}");

    let curr: DateTime<Utc> = curr.into();
    eprintln!("3={curr}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    match test_humantime().await {
        Ok(()) => (),
        Err(e) => eprintln!("error={}", e.to_string()),
    }

    let d: Timestamp = "2024-03-08 01:23:45-00".parse()?;
    let r = d.in_tz("Utc")?.round(jiff::Unit::Day)?;

    eprintln!("d={d} - r={r}");

    Ok(())
}

