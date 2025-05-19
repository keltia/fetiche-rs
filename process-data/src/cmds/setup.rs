//! This task connects to the database and create some useful macros and tables
//! to set our work environment up.
//!
//! >NOTE: THIS IS CLICKHOUSE-SPECIFIC
//!

use std::env;

use clap::Parser;
use eyre::Result;
use klickhouse::Client;

use crate::runtime::Context;

/// Command-line options for setting up the database and environment.
///
/// This structure defines the options for executing the `setup` command,
/// which is responsible for setting up various components of the ACUTE environment.
///
/// ### Options
///
/// - `macros`: If enabled (`-M` or `--macros`), add mathematical macros to the database.
/// - `encounters`: If enabled (`-E` or `--encounters`), create the encounters table to store air-prox points.
/// - `views`: If enabled (`-V` or `--views`), create persistent database views for querying drone and airplane data.
/// - `all`: If enabled (`-a` or `--all`), perform all setup tasks (including macros, encounters, and views).
///
/// ### Example
///
/// Run the setup command to add database macros:
/// ```sh
/// cargo run -- setup --macros
/// ```
///
/// Create all required tables, views, and macros:
/// ```sh
/// cargo run -- setup --all
/// ```
///
/// See also: The `add_macros`, `add_encounters_table`, and `create_views` functions for implementation details.
///
#[derive(Debug, Default, Parser)]
pub struct SetupOpts {
    /// Add only macros.
    #[clap(short = 'M', long)]
    pub macros: bool,
    /// Create encounters (aka calculation) table
    #[clap(short = 'E', long)]
    pub encounters: bool,
    /// Create permanent tables
    #[clap(short = 'V', long)]
    pub views: bool,
    /// Everything.
    #[clap(short = 'a', long)]
    pub all: bool,
}

// -----

#[tracing::instrument(skip(dbh))]
async fn add_macro_dist2d(dbh: &Client) -> Result<()> {
    let r1 = r##"
CREATE FUNCTION dist_2d AS (dx, dy, px, py) ->
  ceil(geoDistance(dx,dy,px,py));
    "##;

    Ok(dbh.execute(r1).await?)
}

#[tracing::instrument(skip(dbh))]
async fn add_macro_dist3d(dbh: &Client) -> Result<()> {
    let r2 = r##"
CREATE FUNCTION dist_3d AS (dx, dy, dz, px, py, pz) ->
  ceil(sqrt(pow(dist_2d(dx,dy,px,py), 2) + pow((dz-pz), 2)));
    "##;

    Ok(dbh.execute(r2).await?)
}

/// Adds mathematical macros to the database for distance calculations.
///
/// ### Details
///
/// This function creates two user-defined functions in the ClickHouse database:
/// - `dist_2d`: Calculates horizontal geodesic distance between two points
/// - `dist_3d`: Calculates three-dimensional distance between two points
///
/// ### Errors
///
/// Returns an error if the macros cannot be created, for example, due to:
/// - Database connection issues
/// - Insufficient privileges
/// - Invalid SQL syntax
/// - Existing functions with the same names
///
/// ### References
///
/// - ClickHouse documentation for user-defined functions
///
#[tracing::instrument(skip(dbh))]
async fn add_macros(dbh: &Client) -> Result<()> {
    add_macro_dist2d(dbh).await?;
    add_macro_dist3d(dbh).await?;
    Ok(())
}

// -----

#[tracing::instrument(skip(dbh))]
async fn remove_macro_dist2d(dbh: &Client) -> Result<()> {
    let r1 = r##"
DROP FUNCTION IF EXISTS dist_2d;
    "##;

    Ok(dbh.execute(r1).await?)
}

#[tracing::instrument(skip(dbh))]
async fn remove_macro_dist3d(dbh: &Client) -> Result<()> {
    let r2 = r##"
DROP FUNCTION IF EXISTS dist_3d;
    "##;

    Ok(dbh.execute(r2).await?)
}

/// Removes mathematical macros from the database.
///
/// This function drops the user-defined functions (`dist_2d` and `dist_3d`)
/// from the ClickHouse database. These functions are used for various distance calculations.
///
/// ### Details
///
/// Removes the following functions:
/// - `dist_2d`: Function for calculating horizontal geodesic distance
/// - `dist_3d`: Function for calculating three-dimensional distance
///
/// ### Errors
///
/// Returns an error if the macros cannot be removed, for example, due to:
/// - Database connection issues
/// - Insufficient privileges
/// - Non-existent functions
///
/// ### References
///
/// - ClickHouse documentation for user-defined functions
///
#[tracing::instrument(skip(dbh))]
async fn remove_macros(dbh: &Client) -> Result<()> {
    remove_macro_dist3d(dbh).await?;
    remove_macro_dist2d(dbh).await?;
    Ok(())
}

// -----

/// Create the `encounters` table to store short air-prox points
///
/// ### Details
///
/// This function creates the `encounters` table (`acute.airplane_prox`) in the database.
/// The table is structured to store proximity data related to drone and airplane encounters
/// within a specified distance threshold.
///
/// - `site`: Represents the site ID where the encounter occurs.
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
/// - `distance_slant_m`: Slant distance in meters between the drone and airplane.
/// - `distance_hor_m`: Horizontal distance in meters between the drone and airplane.
/// - `distance_vert_m`: Vertical distance in meters between the drone and airplane.
/// - `distance_home_m`: Distance in meters between the drone and its home location.
///
/// This command creates the `acute.airplane_prox` table in the database to store the
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
#[tracing::instrument(skip(dbh))]
async fn add_encounters_table(dbh: &Client) -> Result<()> {
    let sq = r##"
CREATE
OR REPLACE TABLE acute.airplane_prox (
  site             INT,
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
  distance_slant_m INT,
  distance_hor_m   INT,
  distance_vert_m  INT,
  distance_home_m  INT,
)
    ENGINE = ReplacingMergeTree PRIMARY KEY (time, journey)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance.';
    "##;

    Ok(dbh.execute(sq).await?)
}

/// Remove the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(dbh))]
async fn drop_encounters_table(dbh: &Client) -> Result<()> {
    let sq = r##"
DROP TABLE IF EXISTS acute.airplane_prox;
    "##;

    Ok(dbh.execute(sq).await?)
}

// -----

#[tracing::instrument(skip(dbh))]
async fn add_pbi_encounters_view(dbh: &Client) -> Result<()> {
    let sq = r##"
CREATE MATERIALIZED VIEW acute.pbi_encounters
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (
SELECT
  en_id,
  installation_id,
  site,
  d.sitename,
  `time`,
  date_trunc('day', ap.time) AS `date`,
  formatDateTime(ap.time, '%T', 'UTC') AS `utc_time`,
  formatDateTime((ap.time + d.timezone * 3600), '%T', 'UTC') AS `local_time`,
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
  prox_mode_a,
  distance_slant_m,
  distance_hor_m,
  distance_vert_m,
  distance_home_m
FROM acute.airplane_prox AS ap, acute.pbi_deployments AS d
LEFT OUTER JOIN acute.sites AS s
ON ap.site = s.id
WHERE s.name = d.sitename
)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance for PBI.';
    "##;

    Ok(dbh.execute(sq).await?)
}

/// Drop the `pbi_encounters` table to store short air-prox points
///
#[tracing::instrument(skip(dbh))]
async fn drop_pbi_encounters_view(dbh: &Client) -> Result<()> {
    let sq = r##"
DROP VIEW IF EXISTS acute.pbi_encounters;
    "##;

    Ok(dbh.execute(sq).await?)
}

// -----

/// Create airplanes view
///
#[tracing::instrument(skip(dbh))]
async fn add_airplanes_view(dbh: &Client) -> Result<()> {
    // Calculations view
    //
    let r1 = r##"
CREATE
OR REPLACE VIEW acute.airplanes
AS
(
    SELECT EmitterCategory,
       (GBS == 1)                     AS GBS,
       ModeA,
       TimeRecPosition                AS time,
       AircraftAddress                AS prox_id,
       Latitude                       AS prox_lat,
       Longitude                      AS prox_lon,
       GeometricAltitude              AS prox_alt,
       FlightLevel                    AS flight_level,
       BarometricVerticalRate         AS baro_vert_rate,
       (GeoVertRateExceeded == '1')   AS geo_vert_exceeded,
       GeometricVerticalRate          AS geo_vert_rate,
       GroundSpeed                    AS ground_speed,
       TrackAngle,
       replaceRegexpOne(prox_callsign, '\'([0-9A-Z]+)\\s*\'', '\\1') AS prox_callsign,
       (AircraftStopped == '1')       AS stopped,
       (GroundTrackValid == '1')      AS GroundTrackValid,
       (GroundHeadingProvided == '1') AS GroundHeadingProvided,
       (MagneticNorth == '1')         AS MagneticNorth,
       SurfaceGroundSpeed,
       SurfaceGroundTrack,
       site
    FROM acute.airplanes_raw AS f
)
    COMMENT 'View for airplanes data.'
"##;

    Ok(dbh.execute(r1).await?)
}

/// Drop airplanes view
///
#[tracing::instrument(skip(dbh))]
async fn drop_airplanes_view(dbh: &Client) -> Result<()> {
    let rm1 = r##"
DROP VIEW IF EXISTS acute.airplanes;
    "##;

    Ok(dbh.execute(rm1).await?)
}

// -----

/// Create drones view
///
#[tracing::instrument(skip(dbh))]
async fn add_drones_view(dbh: &Client) -> Result<()> {
    let r2 = r##"
CREATE MATERIALIZED VIEW acute.drones
    ENGINE = ReplacingMergeTree
    PRIMARY KEY (time, journey)
AS
(
    SELECT
        `journey`,
        `ident`,
        `model`,
        `source`,
        `timestamp`,
        `latitude`,
        `longitude`,
        `altitude`,
        CEIL((CAST(altitude AS Float64)  + compute_height(latitude,longitude))) AS altitude_geo,
        `elevation`,
        `home_lat`,
        `home_lon`,
        `home_height`,
        `speed`,
        `heading`,
        `station_name`,
        `station_latitude`,
        `station_longitude`,
        toUnixTimestamp(timestamp) as time,
        dist_2d(longitude,latitude,home_lon,home_lat) AS home_distance_2d,
        dist_3d(longitude,latitude,elevation,home_lon,home_lat,home_height) AS home_distance_3d
    FROM acute.drones_raw
)
    COMMENT 'View for drones data with distances.'
"##;

    Ok(dbh.execute(r2).await?)
}

#[tracing::instrument(skip(dbh))]
async fn drop_drones_view(dbh: &Client) -> Result<()> {
    let rm2 = r##"
DROP VIEW IF EXISTS acute.drones;
    "##;

    Ok(dbh.execute(rm2).await?)
}
// -----

/// Create PBI-specific drones view
///
#[tracing::instrument(skip(dbh))]
async fn add_pbi_drones_view(dbh: &Client) -> Result<()> {
    let r2b = r##"
CREATE MATERIALIZED VIEW acute.pbi_drones
ENGINE = ReplacingMergeTree
PRIMARY KEY (time, journey) POPULATE
AS (SELECT `journey`,
      `ident`,
      `model`,
      d.installation_id,
      sitename,
      date_trunc('day', dr.timestamp) AS `date`,
      formatDateTime(dr.timestamp, '%T', 'UTC') AS `utc_time`, formatDateTime((timestamp + d.timezone * 3600), '%T', 'UTC') AS local_time,
      dr.latitude AS `drone_lat`,
      dr.longitude AS `drone_lon`,
      CEIL((CAST(dr.altitude AS Float64)  + compute_height(drone_lat,drone_lon))) AS `drone_alt_m`,
      `elevation` AS `elevation_m`,
      `home_lat`,
      `home_lon`,
      `home_height` AS `home_elevation_m`,
      (`speed` / 3.6) AS `speed_m_s`,
      `heading`,
      `station_name`,
      `station_latitude`,
      `station_longitude`,
      toUnixTimestamp(timestamp) as time,
      dist_2d(dr.longitude,dr.latitude,home_lon,home_lat) AS home_distance_2d,
      dist_3d(dr.longitude,dr.latitude,dr.elevation,home_lon,home_lat,home_height) AS home_distance_3d,
      dist_2d(dr.longitude,dr.latitude,station_longitude,station_latitude) AS antenna_distance_2d,
      dist_3d(dr.longitude,dr.latitude,dr.altitude,station_longitude,station_latitude, d.ref_altitude) AS antenna_distance_3d
    FROM acute.drones_raw AS dr LEFT OUTER JOIN acute.pbi_deployments AS d
    ON dr.station_name = d.antenna_name and dr.timestamp between d.start_at and d.end_at
    WHERE sitename = d.sitename AND dr.station_name != 'ASDSTATIONV1'
  )
  COMMENT 'PBI View for drones data with distances.'
"##;

    Ok(dbh.execute(r2b).await?)
}

#[tracing::instrument(skip(dbh))]
async fn drop_pbi_drones_view(dbh: &Client) -> Result<()> {
    let rm2b = r##"
DROP VIEW IF EXISTS acute.pbi_drones;
    "##;
    Ok(dbh.execute(rm2b).await?)
}

// -----

#[tracing::instrument(skip(dbh))]
async fn add_deployments_view(dbh: &Client) -> Result<()> {
    // Deployments tracking view
    //
    let r3 = r##"
 CREATE VIEW acute.deployments
 AS SELECT
    i.id AS install_id,
    i.start_at,
    i.end_at,
    a.type,
    a.name AS antenna_name,
    s.name AS site_name,
    s.timezone AS timezone
 FROM acute.installations AS i, acute.antennas AS a, acute.sites AS s
 WHERE (i.antenna_id = a.id) AND (s.id = i.site_id)
 COMMENT 'Find the site for each drone points.'
    "##;

    Ok(dbh.execute(r3).await?)
}

#[tracing::instrument(skip(dbh))]
async fn drop_deployments_view(dbh: &Client) -> Result<()> {
    let rm3 = r##"
DROP VIEW IF EXISTS acute.deployments;
    "##;

    Ok(dbh.execute(rm3).await?)
}

// -----

#[tracing::instrument(skip(dbh))]
async fn add_pbi_deployments_view(dbh: &Client) -> Result<()> {
    // PBI-specific view
    //
    let r3b = r##"
 CREATE VIEW acute.pbi_deployments
 AS SELECT
    i.id AS installation_id,
    i.start_at,
    i.end_at,
    a.type,
    a.name AS antenna_name,
    s.name AS sitename,
    s.offset AS timezone
 FROM acute.installations AS i, acute.antennas AS a, acute.sites AS s
 WHERE (i.antenna_id = a.id) AND (s.id = i.site_id)
 COMMENT 'Find the site for each drone points for PBI.'
    "##;

    Ok(dbh.execute(r3b).await?)
}

#[tracing::instrument(skip(dbh))]
async fn drop_pbi_deployments_view(dbh: &Client) -> Result<()> {
    let rm4 = r##"
DROP VIEW IF EXISTS acute.pbi_deployments
    "##;

    Ok(dbh.execute(rm4).await?)
}

// -----

#[tracing::instrument(skip(dbh))]
async fn add_airprox_summary_view(dbh: &Client) -> Result<()> {
    let r4 = r##"
CREATE OR REPLACE VIEW airprox_summary AS
(SELECT
        en_id,
        journey,
        drone_id,
        min(distance_slant_m) as distance_slant_m
    FROM
        airplane_prox
    GROUP BY
        en_id, journey, drone_id
    ORDER BY journey)
    COMMENT 'List all encounters ID with the minimum distance.'
    "##;

    Ok(dbh.execute(r4).await?)
}

#[tracing::instrument(skip(dbh))]
async fn drop_airprox_summary_view(dbh: &Client) -> Result<()> {
    let rm4 = r##"
DROP VIEW IF EXISTS acute.airprox_summary
    "##;

    Ok(dbh.execute(rm4).await?)
}

// -----

/// Create various views
///
#[tracing::instrument(skip(dbh))]
async fn create_views(dbh: &Client) -> Result<()> {
    add_airplanes_view(dbh).await?;
    add_deployments_view(dbh).await?;
    add_pbi_deployments_view(dbh).await?;
    add_drones_view(dbh).await?;
    add_pbi_drones_view(dbh).await?;
    add_airprox_summary_view(dbh).await?;
    add_pbi_encounters_view(dbh).await?;

    Ok(())
}

/// Drop all views
///
#[tracing::instrument(skip(dbh))]
async fn drop_views(dbh: &Client) -> Result<()> {
    drop_pbi_encounters_view(dbh).await?;
    drop_airprox_summary_view(dbh).await?;
    drop_pbi_drones_view(dbh).await?;
    drop_pbi_deployments_view(dbh).await?;
    drop_drones_view(dbh).await?;
    drop_deployments_view(dbh).await?;
    drop_airplanes_view(dbh).await?;
    Ok(())
}

/// Create parts or all of the ACUTE environment
///
#[tracing::instrument(skip(ctx))]
pub async fn setup_acute_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dbh = ctx.db().await;
    let dir = ctx.config["datalake"].clone();

    // Move here.
    //
    let _ = env::set_current_dir(&dir);

    if opts.all {
        create_views(&dbh).await?;
        add_macros(&dbh).await?;
        let _ = add_encounters_table(&dbh).await;
    } else {
        if opts.macros {
            add_macros(&dbh).await?;
        }
        if opts.encounters {
            add_encounters_table(&dbh).await?;
        }
    }
    Ok(())
}

/// Cleanup by erasing parts or all
///
#[tracing::instrument(skip(ctx))]
pub async fn cleanup_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dbh = ctx.db().await;
    if opts.all {
        drop_encounters_table(&dbh).await?;
        remove_macros(&dbh).await?;
        drop_views(&dbh).await?;
    } else {
        if opts.macros {
            remove_macros(&dbh).await?;
        }
        if opts.encounters {
            drop_encounters_table(&dbh).await?;
        }
        if opts.views {
            drop_views(&dbh).await?;
        }
    }

    Ok(())
}

/// Bootstrapping is a combination of both cleanup/setup to start with a clean slate
///
#[tracing::instrument(skip(ctx))]
pub async fn bootstrap(ctx: &Context) -> Result<()> {
    // Remove everything
    //
    let opts = &SetupOpts {
        all: true,
        ..SetupOpts::default()
    };
    cleanup_environment(ctx, opts).await?;

    // Fiat Lux
    //
    setup_acute_environment(ctx, opts).await?;

    Ok(())
}
