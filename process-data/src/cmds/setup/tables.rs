//! Setup needed tables in ClickHouse.
//!

use crate::cmds::{load_query, DBVars};
use crate::runtime::Context;

/// Create the `encounters` table to store short air-prox points
///
/// ### Details
///
/// This function creates the `encounters` table (`airplane_prox`) in the database.
/// The table is structured to store proximity data related to drone and airplane encounters
/// within a specified distance threshold.
///
/// - `site`: Represents the site ID where the encounter occurs.
/// - `sitename`: Name of the site where the encounter occurs.
/// - `en_id`: Unique identifier for the encounter.
/// - `time`: Timestamp indicating when the encounter occurred.
/// - `journey`: Identifier for the journey or flight path of the drone.
/// - `drone_id`: The unique identifier for the drone.
/// - `model`: Drone model information.
/// - `drone_lat`: Latitude of the drone.
/// - `drone_lon`: Longitude of the drone.
/// - `drone_alt_m`: Altitude of the drone in meters.
/// - `drone_height_m`: Drone height in meters above ground level.
/// - `prox_callsign`: Callsign of the nearby airplane.
/// - `prox_id`: Unique identifier of the airplane.
/// - `prox_lat`: Latitude of the airplane.
/// - `prox_lon`: Longitude of the airplane.
/// - `prox_alt_m`: Altitude of the airplane in meters.
/// - `prox_mode_a`: Squawk code of the aircraft.
/// - `prox_ecat`: Category of the airplane (e.g., GBS, UAS, etc. EmitterCategory from ADS-B).
/// - `distance_slant_m`: Slant distance in meters between the drone and airplane.
/// - `distance_hor_m`: Horizontal distance in meters between the drone and airplane.
/// - `distance_vert_m`: Vertical distance in meters between the drone and airplane.
/// - `distance_home_m`: Distance in meters between the drone and its home location.
///
/// This command creates the `airplane_prox` table in the database to store the
/// described data points for encounters.
///
/// ### Errors
///
/// Returns an error if the table cannot be created. Possible causes include:
/// - Database connection issues.
/// - Insufficient privileges to create tables in the database.
/// - SQL syntax or schema errors.
///
/// ### References
///
/// - ClickHouse documentation for managing tables.
#[tracing::instrument(skip(ctx))]
pub async fn add_encounters_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let sq = load_query(r##"
CREATE TABLE IF NOT EXISTS {workdb}.airplane_prox (
  site_id          INT,
  sitename         VARCHAR,
  en_id            VARCHAR,
  time             TIMESTAMP,
  journey          INT,
  drone_id         VARCHAR,
  model            VARCHAR,
  drone_lat        FLOAT,
  drone_lon        FLOAT,
  drone_alt_m      FLOAT,
  drone_height_m   FLOAT,
  prox_callsign    VARCHAR,
  prox_id          VARCHAR,
  prox_lat         FLOAT,
  prox_lon         FLOAT,
  prox_alt_m       FLOAT,
  prox_mode_a      VARCHAR,
  prox_ecat        INT,
  distance_slant_m INT,
  distance_hor_m   INT,
  distance_vert_m  INT,
  distance_home_m  INT,
  station_name     VARCHAR
)
    ENGINE = ReplacingMergeTree PRIMARY KEY (time, journey)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance.';
    "##, &dbvars)?;

    Ok(dbh.execute(sq).await?)
}

/// Remove the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(ctx))]
pub async fn drop_encounters_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let sq = load_query(r##"
DROP TABLE IF EXISTS {workdb}.airplane_prox;
    "##, &dbvars)?;

    Ok(dbh.execute(sq).await?)
}

// ----- Record-related table

#[tracing::instrument(skip(ctx))]
pub async fn add_daily_stats_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let crt = load_query(r##"
CREATE TABLE IF NOT EXISTS {workdb}.daily_stats (
  day DATE,
  site_id INT,
  site_name VARCHAR,
  status INT NOT NULL,
  stats VARCHAR,
  comment VARCHAR,
)
ENGINE = ReplacingMergeTree PRIMARY KEY (day, site_name)
COMMENT 'Records the run history for all sites every day.';
    "##, &dbvars)?;

    Ok(dbh.execute(crt).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_daily_stats_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let crt = load_query(r##"
DROP TABLE {workdb}.daily_stats IF EXISTS {workdb}.daily_stats
    "##, &dbvars)?;

    Ok(dbh.execute(crt).await?)
}

