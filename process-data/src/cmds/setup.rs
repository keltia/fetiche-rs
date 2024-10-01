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

use crate::config::Context;

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

/// Macros :
///
/// - dist_2d       geodesic distance between two points
/// - dist_3d       3D distance based on geodesic
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
    info!("Creating the airplanes and drones views.");

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
CREATE OR REPLACE VIEW acute.drones AS
(
    SELECT
        *,
        toUnixTimestamp(timestamp) as time,
        dist_2d(longitude,latitude,home_lon,home_lat) AS home_distance_2d,
        dist_3d(longitude,latitude,elevation,home_lon,home_lat,home_height) AS home_distance_3d
    FROM acute.drones_raw
)
    COMMENT 'View for drones data with distances.'
"##;

    let r3 = r##"
CREATE OR REPLACE VIEW acute.what_where_when AS
(
    SELECT
        i.id AS install_id,
        i.start_at,
        i.end_at,
        a.type,
        a.name,
        s.name
    FROM installations AS i, antennas AS a, sites AS s
    WHERE (i.antenna_id = a.id) AND (s.id = i.site_id)
)
    COMMENT 'Find the site for each drone points.'
    "##;

    dbh.execute(r1).await?;
    dbh.execute(r2).await?;
    dbh.execute(r3).await?;
    Ok(())
}

/// Remove both views
///
#[tracing::instrument(skip(dbh))]
async fn drop_views(dbh: &Client) -> Result<()> {
    info!("Dropping airplanes and drones views.");

    let rm1 = r##"
DROP VIEW IF EXISTS acute.airplanes;
    "##;

    let rm2 = r##"
DROP VIEW IF EXISTS acute.drones;
    "##;

    let rm3 = r##"
DROP VIEW IF EXISTS acute.what_where_when;
    "##;

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
