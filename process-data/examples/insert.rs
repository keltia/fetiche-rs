use clickhouse::{Client, Row};
use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Row)]
struct Item {
    id: u32,
    bar: String,
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
    CREATE TABLE IF NOT EXISTS foobar (
         id        UInt32,
         bar       Nullable(VARCHAR),
    ) ENGINE = Memory
    "##;

    let mut data = Vec::<Item>::new();
    data.push(Item { id: 0, bar: "h".into() });
    data.push(Item { id: 1, bar: "g".into() });
    data.push(Item { id: 2, bar: "f".into() });
    data.push(Item { id: 3, bar: "e".into() });
    data.push(Item { id: 4, bar: "d".into() });
    data.push(Item { id: 5, bar: "c".into() });
    data.push(Item { id: 6, bar: "b".into() });
    data.push(Item { id: 7, bar: "a".into() });
    data.push(Item { id: 8, bar: "".into() });

    let _ = dbh.query(ddl).execute().await?;

    let mut batch = dbh.insert("foobar")?;
    for item in data.iter() {
        let _ = batch.write(item).await?;
    }
    let _ = batch.end().await?;

    Ok(())
}