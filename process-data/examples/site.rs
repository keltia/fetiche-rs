use eyre::Result;
use fetiche_common::init_logging;
use klickhouse::{Client, ClientOptions, Progress, QueryBuilder, RawRow, Row, Uuid};
use tracing::{debug, trace};

#[derive(Debug, Row)]
struct Ans {
    id: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let name = std::env::var("CLICKHOUSE_DB")?;
    let user = std::env::var("CLICKHOUSE_USER")?;
    let pass = std::env::var("CLICKHOUSE_PASSWD")?;
    let endpoint = std::env::var("KLICKHOUSE_URL")?;

    init_logging("site", false, false, None)?;

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

    // Retrieve and display query progress events
    //
    let mut progress = client.subscribe_progress();
    let progress_task = tokio::task::spawn(async move {
        let mut current_query = Uuid::nil();
        let mut progress_total = Progress::default();
        while let Ok((query, progress)) = progress.recv().await {
            if query != current_query {
                progress_total = Progress::default();
                current_query = query;
            }
            progress_total += progress;
            println!(
                "Progress on query {}: {}/{} {:.2}%",
                query,
                progress_total.read_rows,
                progress_total.new_total_rows_to_read,
                100.0 * progress_total.read_rows as f64
                    / progress_total.new_total_rows_to_read as f64
            );
        }
    });

    let site = "AUS";
    let q = QueryBuilder::new("SELECT id FROM sites WHERE name = $1;").arg(site);

    trace!("Get site_id for {}", site);
    let mut id_site = client.query_one::<RawRow>(q).await?;
    dbg!(&id_site);
    let id_site: i32 = id_site.get(0);
    dbg!(&id_site);
    debug!("site_id for {site} is {:?}", id_site);

    drop(client);
    progress_task.await?;

    Ok(())
}
