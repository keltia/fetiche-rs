// -----

/// Small internal module for clickhouse data fetching
///
use chrono::{DateTime, Datelike, Utc};
use eyre::Result;
use fetiche_common::DateOpts;
use klickhouse::{Client, QueryBuilder, RawRow, Row};
use serde::Serialize;
use tracing::{debug, trace};


/// Main struct for data points, both drone and plane
///
#[derive(Clone, Debug, Row, Serialize)]
pub(crate) struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

/// What we need from the `airplane_prox` table.
///
#[derive(Clone, Debug, Row, Serialize)]
pub(crate) struct Encounter {
    pub en_id: String,
    pub journey: i32,
    pub drone_id: String,
    pub prox_id: String,
    pub prox_callsign: String,
}

#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_drones(
    client: &Client,
    journey: i32,
    drone_id: &str,
) -> Result<Vec<DataPoint>> {
    // Fetch drone points
    //
    let rpp = r##"
SELECT
  toDateTime(timestamp) as timestamp,
  latitude,
  longitude,
  toFloat64(altitude) AS altitude
FROM drones
WHERE
journey = $1 AND
ident = $2
ORDER BY timestamp
    "##;

    let q = QueryBuilder::new(rpp).arg(journey).arg(drone_id);
    let drones = client.query_collect::<DataPoint>(q).await?;
    trace!("Found {} drone points for en_id {}", drones.len(), drone_id);

    debug!("drones={:?}", drones);

    Ok(drones)
}

#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_planes(
    client: &Client,
    prox_id: &str,
    first: DateTime<Utc>,
    last: DateTime<Utc>,
) -> Result<Vec<DataPoint>> {
    // Fetch plane points
    //
    let rdp = r##"
SELECT
  time,
  prox_lat AS latitude,
  prox_lon AS longitude,
  prox_alt AS altitude
FROM airplanes
WHERE
  prox_id = $1 AND
  time BETWEEN $2 AND $3
ORDER BY time
    "##;

    let q = QueryBuilder::new(rdp).arg(prox_id).arg(first).arg(last);
    let planes = client.query_collect::<DataPoint>(q).await?;
    trace!("Found {} plane points for id {}", planes.len(), prox_id);

    debug!("planes={:?}", planes);

    Ok(planes)
}

#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_one_encounter(client: &Client, id: &str) -> Result<Encounter> {
    // Fetch the drone & airplane IDs
    //
    let rp = r##"
SELECT
  en_id, journey, drone_id, prox_callsign, prox_id
FROM airplane_prox
WHERE en_id = $1
    "##;
    let q = QueryBuilder::new(rp).arg(id);
    let res = client.query_one::<Encounter>(q).await?;

    Ok(res)
}

#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_all_en_id(client: &Client) -> Result<Vec<String>> {
    let r = r##"
SELECT
  en_id
FROM
  airprox_summary
ORDER BY
  en_id
    "##;
    let list = client
        .query_collect::<RawRow>(r)
        .await?
        .iter_mut()
        .map(|e| e.get(0))
        .collect::<Vec<String>>();
    Ok(list)
}

/// Select a list of en_id on a date specification.
///
#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_encounters_on(client: &Client, date: DateOpts) -> Result<Vec<String>> {
    let (begin, _) = DateOpts::parse(date)?;
    let en_id_pat = format!("{:4}{:02}{:02}", begin.year(), begin.month(), begin.day());

    debug!("en_id_pat={}", en_id_pat);
    let r = format!(r##"
SELECT
  en_id
FROM
  airprox_summary
WHERE
  en_id LIKE '%{en_id_pat}%'
ORDER BY
  en_id
        "##);

    let list = client
        .query_collect::<RawRow>(r)
        .await?
        .iter_mut()
        .map(|e| e.get(0))
        .collect::<Vec<String>>();
    Ok(list)
}
