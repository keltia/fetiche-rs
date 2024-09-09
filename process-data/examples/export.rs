use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use eyre::Result;
use fetiche_common::{close_logging, init_logging};
use klickhouse::{Client, ClientOptions, QueryBuilder, Row};
use tracing::trace;

#[derive(Debug, Row)]
struct Ans {
    id: u32,
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
    let endpoint = std::env::var("KLICKHOUSE_URL")?;

    init_logging("export-encounters", false, true, true)?;

    trace!("Create connection.");
    let client = Client::connect(
        endpoint,
        ClientOptions {
            username: user,
            password: pass,
            default_database: name,
        },
    )
        .await?;

    let fname = opts.fname.clone();
    let r = format!(
        r##"
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
  INTO OUTFILE '{}' FORMAT CSVWithNames
        "##,
        fname
    );
    trace!("q={r}");

    let q = QueryBuilder::new(&r);
    let _ = client.execute(q).await?;

    close_logging();
    Ok(())
}
