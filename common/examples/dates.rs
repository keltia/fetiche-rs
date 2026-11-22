use eyre::Result;
use jiff::{SignedDuration, Timestamp};

async fn test_jiff() -> Result<()> {
    let base = "2024-03-08T00:00:00Z";

    let curr: Timestamp = base.parse()?;
    eprintln!("1={curr:?}");

    let added: SignedDuration = "3h 5m".parse()?;
    let curr = curr.checked_add(added)?;
    eprintln!("2={curr:?}");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    match test_jiff().await {
        Ok(()) => (),
        Err(e) => eprintln!("error={e}"),
    }

    let d: Timestamp = "2024-03-08 01:23:45-00".parse()?;
    let r = d.in_tz("Utc")?.round(jiff::Unit::Day)?;

    eprintln!("d={d} - r={r}");

    Ok(())
}
