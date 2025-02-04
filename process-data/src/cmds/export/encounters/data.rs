//! # Data Export Module
//!
//! This module handles data fetching and processing for export purposes, 
//! specifically related to drone and proximate aircraft encounters. 
//! It interacts with a ClickHouse database to retrieve relevant data points 
//! and encounter records based on specific criteria.
//!
//! ## Key Features
//! - Fetch drone data points for specific journeys and drone identifiers.
//! - Fetch proximate aircraft data points within specified time ranges.
//! - Retrieve detailed encounter records between drones and proximate aircraft.
//!
//! ## Dependencies
//! This module utilizes the following crates:
//! - `chrono`: For handling datetime operations.
//! - `eyre`: For error handling and propagation.
//! - `klickhouse`: For constructing and executing ClickHouse database queries.
//! - `serde`: For serialization support.
//! - `tracing`: For logging and instrumentation.
//!
use chrono::{DateTime, Datelike, Utc};
use eyre::Result;
use fetiche_common::DateOpts;
use klickhouse::{Client, QueryBuilder, RawRow, Row};
use serde::Serialize;
use tracing::{debug, trace};


/// This struct represents a single data point with positional and temporal information.
///
/// # Fields
///
/// * `timestamp` - The timestamp of the data point in UTC.
/// * `latitude` - Latitude position (in degrees) of the data point.
/// * `longitude` - Longitude position (in degrees) of the data point.
/// * `altitude` - Altitude of the data point (in meters).
///
/// This struct is typically used to represent the location and altitude of either a drone or
/// a proximate aircraft (plane) at a specific moment in time.
/// 
#[derive(Clone, Debug, Row, Serialize)]
pub(crate) struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
}

/// Represents an encounter between a drone and a proximate aircraft.
///
/// # Fields
///
/// * `en_id` - Unique identifier for the encounter.
/// * `journey` - Identifier for the journey during which the encounter occurred.
/// * `timestamp` - The timestamp of the encounter in UTC.
/// * `drone_id` - Unique identifier of the drone involved in the encounter.
/// * `drone_lat` - The latitude position of the drone during the encounter.
/// * `drone_lon` - The longitude position of the drone during the encounter.
/// * `drone_alt_m` - The altitude of the drone in meters during the encounter.
/// * `prox_id` - Unique identifier for the proximate aircraft involved in the encounter.
/// * `prox_callsign` - The callsign of the proximate aircraft.
/// * `prox_lat` - The latitude position of the proximate aircraft.
/// * `prox_lon` - The longitude position of the proximate aircraft.
/// * `prox_alt_m` - The altitude of the proximate aircraft in meters.
///
#[derive(Clone, Debug, Row, Serialize)]
pub(crate) struct Encounter {
    pub en_id: String,
    pub journey: i32,
    pub timestamp: DateTime<Utc>,
    pub drone_id: String,
    pub drone_lat: f32,
    pub drone_lon: f32,
    pub drone_alt_m: f32,
    pub prox_id: String,
    pub prox_callsign: String,
    pub prox_lat: f32,
    pub prox_lon: f32,
    pub prox_alt_m: f32,
}

/// Fetch data points for a specific drone ID and journey from the database.
///
/// # Arguments
///
/// * `client` - A reference to the ClickHouse client used for database interaction.
/// * `journey` - The journey identifier to filter drone data.
/// * `drone_id` - The drone identifier to filter drone data.
///
/// # Returns
///
/// A `Result` containing a vector of `DataPoint` structs if successful, or an error if one occurs.
///
/// # Database Query
///
/// This function executes a SQL query to fetch timestamp, latitude, longitude, and altitude
/// data for drones matching the given journey and drone ID. The altitude is converted to a floating-point
/// number (f64), and the results are ordered by timestamp.
///
/// # Examples
///
/// ```rust
/// let drones = fetch_drones(&client, 123, "drone_001").await?;
/// ```
///
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

/// Fetch data points for a specific proximate aircraft (plane) ID within a time range.
///
/// # Arguments
///
/// * `client` - A reference to the ClickHouse client used for database interaction.
/// * `prox_id` - The proximate aircraft identifier to filter plane data.
/// * `first` - The start of the time range used to filter data (inclusive).
/// * `last` - The end of the time range used to filter data (inclusive).
///
/// # Returns
///
/// A `Result` containing a vector of `DataPoint` structs if successful, or an error if one occurs.
///
/// # Database Query
///
/// This function executes a SQL query to fetch timestamp, latitude, longitude, and altitude
/// data for proximate aircraft matching the given ID and time range. The altitude is already
/// provided in meters and appropriately mapped.
///
/// # Examples
///
/// ```rust
/// let planes = fetch_planes(&client, "prox_001", chrono::Utc::now() - chrono::Duration::hours(1), chrono::Utc::now()).await?;
/// ```
///
#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_planes(
    client: &Client,
    prox_id: &str,
    first: DateTime<Utc>,
    last: DateTime<Utc>,
) -> Result<Vec<DataPoint>> {
    // Fetch plane points
    //
    // We need to convert altitude into meters.
    //
    let rdp = r##"
SELECT
  time,
  prox_lat AS latitude,
  prox_lon AS longitude,
  prox_alt_m AS altitude
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

/// Fetch a specific encounter record from the database based on its unique identifier.
///
/// # Arguments
///
/// * `client` - A reference to the ClickHouse client used for database interaction.
/// * `id` - The unique identifier of the encounter to fetch.
///
/// # Returns
///
/// A `Result` containing an `Encounter` struct if the record is found, or an error otherwise.
///
/// # Database Query
///
/// This function executes a SQL query to fetch the encounter record from the `airplane_prox`
/// table that matches the given `en_id`. The fields include information about the encounter,
/// such as the journey ID, drone and proximate aircraft details (e.g., latitude, longitude,
/// altitude, and callsign).
///
#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_one_encounter(client: &Client, id: &str) -> Result<Encounter> {
    // Fetch the drone & airplane IDs
    //
    let rp = r##"
SELECT
  en_id, journey, time, drone_id, drone_lat, drone_lon, drone_alt_m, prox_id, prox_callsign, prox_lat, prox_lon, truncate(prox_alt_m) AS prox_alt_m
FROM airplane_prox
WHERE en_id = $1
    "##;
    let q = QueryBuilder::new(rp).arg(id);
    let res = client.query_one::<Encounter>(q).await?;

    Ok(res)
}

/// Fetch all encounter IDs from the database, ordered by their unique identifier.
///
/// # Arguments
///
/// * `client` - A reference to the ClickHouse client used for database interaction.
///
/// # Returns
///
/// A `Result` containing a vector of encounter IDs (`String`) if successful, or an error otherwise.
///
/// # Database Query
///
/// This function executes a SQL query to fetch all encounter IDs (`en_id`) from the
/// `airprox_summary` table and orders them by `en_id`.
///
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

/// Fetch encounter IDs for a specific date or date range.
///
/// # Arguments
///
/// * `client` - A reference to the ClickHouse client used for database interaction.
/// * `date` - A `DateOpts` struct specifying the target date or date range.
///
/// # Returns
///
/// A `Result` containing a vector of encounter IDs (`String`) if successful, or an error otherwise.
///
/// # Database Query
///
/// This function constructs a SQL query to fetch all encounter IDs from the `airprox_summary` table
/// matching a date pattern derived from the input `DateOpts`. The results are ordered by `en_id`.
///
#[tracing::instrument(skip(client))]
pub(crate) async fn fetch_encounters_on(client: &Client, date: DateOpts) -> Result<Vec<String>> {
    let (begin, _) = DateOpts::parse(date)?;
    let en_id_pat = format!("{:4}{:02}{:02}", begin.year(), begin.month(), begin.day());

    debug!("en_id_pat={}", en_id_pat);
    let r = format!(
        r##"
SELECT
  en_id
FROM
  airprox_summary
WHERE
  en_id LIKE '%{en_id_pat}%'
ORDER BY
  en_id
        "##
    );

    let list = client
        .query_collect::<RawRow>(r)
        .await?
        .iter_mut()
        .map(|e| e.get(0))
        .collect::<Vec<String>>();
    Ok(list)
}
