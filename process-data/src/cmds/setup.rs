//! This task connects to the database and create some useful macros and tables
//! to set our work environment up.
//!
//! >NOTE: THIS IS CLICKHOUSE-SPECIFIC
//!

use std::env;

use clap::Parser;
use eyre::Result;
use klickhouse::Client;
use tracing::info;

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

/// Adds mathematical macros to the database for geodesic distance calculations.
///
/// This function creates two user-defined functions (`dist_2d` and `dist_3d`) in the
/// ClickHouse database. These functions are used to calculate the horizontal (2D)
/// and three-dimensional (3D) distances between points, based on their geodesic locations.
///
/// ### Details
///
/// - `dist_2d`: Calculates the horizontal geodesic distance between two points.
/// - `dist_3d`: Calculates the three-dimensional distance using geodesic positioning
///   (includes vertical elevation differences).
///
/// ### SQL Implementation
///
/// - `dist_2d`: Uses the `geoDistance` function to compute the geodesic distance between two points.
/// - `dist_3d`: Combines the geodesic distance in 2D (`dist_2d`) and the vertical difference using
///   the Pythagorean theorem (`sqrt(dx^2 + dz^2)`).
///
/// ### Example
///
/// ```rust
/// add_macros(&dbh).await?;
/// ```
///
/// This command will add the above-described macros (`dist_2d` and `dist_3d`) to the database.
///
/// ### Errors
///
/// Returns an error if the macros cannot be created, for example, due to database connection issues
/// or insufficient privileges.
///
/// ### References
///
/// - ClickHouse documentation for user-defined functions
///
#[tracing::instrument(skip(dbh))]
async fn add_macros(dbh: &Client) -> Result<()> {
    eprintln!("Adding functions.");

    let r1 = r##"
CREATE FUNCTION dist_2d AS (dx, dy, px, py) ->
  ceil(geoDistance(dx,dy,px,py));
    "##;
    let r2 = r##"
CREATE FUNCTION dist_3d AS (dx, dy, dz, px, py, pz) ->
  ceil(sqrt(pow(dist_2d(dx,dy,px,py), 2) + pow((dz-pz), 2)));
    "##;

    dbh.execute(r1).await?;
    dbh.execute(r2).await?;

    Ok(())
}

/// Remove the `dist_2d` and `dist_3d` user-defined functions from the database.
///
/// ### Details
///
/// This function will execute SQL commands to drop the user-defined functions
/// that were previously created for calculating geodesic distances:
///
/// - `dist_2d`: The 2D geodesic distance function.
/// - `dist_3d`: The 3D geodesic distance function.
///
/// ### Usage
///
/// This function should be called when the macros are no longer needed or as part of cleanup procedures:
///
/// ```rust
/// remove_macros(&dbh).await?;
/// ```
///
/// ### Errors
///
/// Returns an error if the user-defined functions cannot be dropped, for example, due to:
/// - Database connection issues.
/// - Insufficient privileges.
///
/// ### References
///
/// - ClickHouse documentation for managing user-defined functions.
///
#[tracing::instrument(skip(dbh))]
async fn remove_macros(dbh: &Client) -> Result<()> {
    eprintln!("Removing macros.");

    let r1 = r##"
DROP FUNCTION IF EXISTS dist_2d;
    "##;

    let r2 = r##"
DROP FUNCTION IF EXISTS dist_3d;
    "##;

    dbh.execute(r1).await?;
    dbh.execute(r2).await?;
    Ok(())
}

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
/// ### Example
///
/// ```rust
/// add_encounters_table(&dbh).await?;
/// ```
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
    info!("Adding airplane_prox table.");

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
    info!("Removing encounters table.");

    let sq = r##"
DROP TABLE IF EXISTS acute.airplane_prox;
    "##;

    Ok(dbh.execute(sq).await?)
}

/// Create the two main raw tables
///
#[tracing::instrument(skip(dbh))]
async fn create_views(dbh: &Client) -> Result<()> {
    info!("Creating the airplanes, drones and proximy views.");

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

    let r4 = r##"
CREATE OR REPLACE VIEW airprox_summary AS
(
    SELECT
        en_id,
        journey,
        drone_id,
        min(distance_slant_m) as distance_slant_m
    FROM
        airplane_prox
    GROUP BY
        en_id,journey,drone_id
    ORDER BY journey
  )
    COMMENT 'List all encounters ID with the minimum distance.'
    "##;

    dbh.execute(r1).await?;
    dbh.execute(r2).await?;
    dbh.execute(r3).await?;
    dbh.execute(r3b).await?;
    dbh.execute(r4).await?;
    Ok(())
}

/// Remove both views
///
#[tracing::instrument(skip(dbh))]
async fn drop_views(dbh: &Client) -> Result<()> {
    info!("Dropping all views.");

    let rm1 = r##"
DROP VIEW IF EXISTS acute.airplanes;
    "##;

    let rm2 = r##"
DROP VIEW IF EXISTS acute.drones;
    "##;

    let rm3 = r##"
DROP VIEW IF EXISTS acute.deployments;
    "##;

    let rm3b = r##"
DROP VIEW IF EXISTS acute.pbi_deployments;
    "##;

    let rm4 = r##"
DROP VIEW IF EXISTS acute.airprox_summary
    "##;

    dbh.execute(rm4).await?;
    dbh.execute(rm3b).await?;
    dbh.execute(rm3).await?;
    dbh.execute(rm2).await?;
    dbh.execute(rm1).await?;
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
