use clickhouse::{Client, Row};
use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Row)]
struct Tc {
    en_id: String,
    journey: u32,
    drone_id: String,
    callsign: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let name = std::env::var("CLICKHOUSE_DB")?;
    let user = std::env::var("CLICKHOUSE_USER")?;
    let pass = std::env::var("CLICKHOUSE_PASSWD")?;
    let endpoint = std::env::var("CLICKHOUSE_URL")?;

    let dbh = Client::default()
        .with_url(endpoint.clone())
        .with_database(&name)
        .with_user(&user)
        .with_password(&pass);

    let ddl = r##"
CREATE TABLE IF NOT EXISTS new_ids (
    drone_id VARCHAR,
    callsign VARCHAR,
    journey INT,
    en_id VARCHAR,
) ENGINE = Memory
    "##;

    let mut data = Vec::<Tc>::new();
    data.push(Tc { en_id: "BRU-20231128-37913-0".into(), journey: 37913, drone_id: "687CKAU0011JYP".into(), callsign: "BEL8DK".into() });
    data.push(Tc { en_id: "BRU-20231128-37907-1".into(), journey: 37907, drone_id: "F4XF82376006N14Q".into(), callsign: "BIRD380".into() });
    data.push(Tc { en_id: "BRU-20231128-37907-2".into(), journey: 37907, drone_id: "F4XF82376006N14Q".into(), callsign: "RYR8XL".into() });
    data.push(Tc { en_id: "BRU-20231128-37907-3".into(), journey: 37907, drone_id: "F4XF82376006N14Q".into(), callsign: "SKEY420".into() });
    data.push(Tc { en_id: "BRU-20231128-37907-4".into(), journey: 37907, drone_id: "F4XF82376006N14Q".into(), callsign: "BEL5WG".into() });
    data.push(Tc { en_id: "BRU-20231128-37907-5".into(), journey: 37907, drone_id: "F4XF82376006N14Q".into(), callsign: "SKEY611".into() });
    data.push(Tc { en_id: "BRU-20231128-37907-6".into(), journey: 37907, drone_id: "F4XF82376006N14Q".into(), callsign: "AEE2BR".into() });

    let _ = dbh.query(ddl).execute().await?;

    let mut batch = dbh.insert("new_ids")?;
    for item in data.iter() {
        let _ = batch.write(item).await?;
    }
    let _ = batch.end().await?;

    Ok(())
}