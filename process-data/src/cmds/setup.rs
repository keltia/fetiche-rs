//! This task connects to the database and create some useful macros and tables
//! to set our work environment up.
//!
//! >NOTE: THIS IS CLICKHOUSE-SPECIFIC
//!

use std::env;

use clap::Parser;
use clickhouse::Client;
use eyre::Result;
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
    /// Create sequences
    #[clap(short = 'S', long)]
    pub sequences: bool,
    /// Create permanent tables
    #[clap(short = 'V', long)]
    pub tables: bool,
    /// Everything.
    #[clap(short = 'a', long)]
    pub all: bool,
}

/// Macros :
///
/// - nm_to_deg     convert nautical miles into degrees
/// - deg_to_m      convert degrees into meters
/// - m_to_deg      and back to degrees
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

    let _ = dbh.query(r1).execute().await?;
    let _ = dbh.query(r2).execute().await?;
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

    let _ = dbh.query(r1).execute().await?;
    let _ = dbh.query(r2).execute().await?;
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
  site             VARCHAR,
  en_id            VARCHAR,
  time             TIMESTAMP,
  journey          INT,
  drone_id         VARCHAR,
  model            VARCHAR,
  drone_lon        FLOAT,
  drone_lat        FLOAT,
  drone_alt_m      FLOAT,
  drone_height_m   FLOAT,
  prox_callsign    VARCHAR,
  prox_id          VARCHAR,
  prox_lon         FLOAT,
  prox_lat         FLOAT,
  prox_alt_m       FLOAT,
  distance_slant_m INT,
  distance_hor_m   INT,
  distance_vert_m  INT,
  distance_home_m  INT,
)
    ENGINE = MergeTree PRIMARY KEY (time, journey)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance.';
    "##;

    Ok(dbh.query(sq).execute().await?)
}

/// Remove the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(dbh))]
async fn drop_encounters_table(dbh: &Client) -> Result<()> {
    info!("Removing encounters table.");

    let sq = r##"
DROP TABLE IF EXISTS acute.airplane_prox;
    "##;

    Ok(dbh.query(sq).execute().await?)
}

/// Add the sequences we need
///
#[tracing::instrument(skip(_dbh))]
async fn add_sequences(_dbh: &Client) -> Result<()> {
    info!("Adding sequences");

    Ok(())
}

/// Add the sequences we need
///
#[tracing::instrument(skip(_dbh))]
async fn drop_sequences(_dbh: &Client) -> Result<()> {
    info!("Adding sequences");

    Ok(())
}

/// Create the two main views
///
/// Assume that the current directory is the datalake so that we use relative paths
/// for `read_parquet()`.
///
#[tracing::instrument(skip(dbh))]
async fn create_tables(dbh: &Client) -> Result<()> {
    info!("Creating the airplanes and drones tables.");

    let r1 = r##"
CREATE
OR REPLACE TABLE acute.airplanes (
    EmitterCategory        INT,
    GBS                    BOOLEAN,
    ModeA                  VARCHAR,
    time                   TIMESTAMP,
    prox_id                VARCHAR,
    prox_lat               DOUBLE,
    prox_lon               DOUBLE,
    prox_alt               DOUBLE,
    flight_level           DOUBLE,
    baro_vert_rate         DOUBLE,
    geo_vert_exceeded      BOOLEAN,
    geo_vert_rate          DOUBLE,
    ground_speed           DOUBLE,
    TrackAngle             DOUBLE,
    prox_callsign          VARCHAR,
    stopped                BOOLEAN,
    GroundTrackValid       BOOLEAN,
    GroundHeadingProvided  BOOLEAN,
    MagneticNorth          BOOLEAN,
    SurfaceGroundSpeed     DOUBLE,
    SurfaceGroundTrack     DOUBLE,
    site                   VARCHAR,
) ENGINE = MergeTree PRIMARY KEY (site, time, prox_id)
    COMMENT 'Main table for ADS-B positions.';
"##;

        let r2 = r##"
CREATE
OR REPLACE acute.drones (
    journey            INT,
    ident              VARCHAR,
    model              VARCHAR,
    source             VARCHAR,
    location           INT,
    timestamp          TIMESTAMP,
    latitude           DOUBLE,
    longitude          DOUBLE,
    altitude           INTEGER,
    elevation          INT,
    gps                INTEGER,
    rssi               INTEGER,
    home_lat           DOUBLE,
    home_lon           DOUBLE,
    home_height        INT,
    speed              INT,
    heading            INT,
    station_name       VARCHAR,
    station_latitude   DOUBLE,
    station_longitude  DOUBLE,
    time               INT,
    year               INT,
    month              INT
    home_distance_2d   DOUBLE,
    home_distance_3d   DOUBLE,
)
    ENGINE = MergeTree PRIMARY KEY (journey, timestamp)
    COMMENT 'Drone positions for all sites.'
"##;

    let _ = dbh.query(r1).execute().await?;
    let _ = dbh.query(r2).execute().await?;
    Ok(())
}

/// Remove both views
///
#[tracing::instrument(skip(dbh))]
async fn drop_tables(dbh: &Client) -> Result<()> {
    info!("Dropping airplanes and drones views.");

    let rm1 = r##"
DROP TABLE IF EXISTS acute.airplanes;
    "##;

    let rm2 = r##"
DROP TABLE IF EXISTS acute.drones;
    "##;

    let _ = dbh.query(rm1).execute().await?;
    let _ = dbh.query(rm2).execute().await?;
    Ok(())
}

/// Create parts or all of the ACUTE environment
///
#[tracing::instrument(skip(ctx))]
pub async fn setup_acute_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dbh = ctx.db();
    let dir = ctx.config["datalake"].clone();

    // Move here.
    //
    let _ = env::set_current_dir(&dir);

    if opts.all {
        add_sequences(&dbh).await?;
        create_tables(&dbh).await?;
        add_macros(&dbh).await?;
        add_encounters_table(&dbh).await;
    } else {
        if opts.sequences {
            add_sequences(&dbh).await?;
        }
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
    let dbh = ctx.db();
    if opts.all {
        drop_encounters_table(&dbh).await?;
        remove_macros(&dbh).await?;
        drop_tables(&dbh).await?;
        drop_sequences(&dbh).await?;
    } else {
        if opts.macros {
            remove_macros(&dbh).await?;
        }
        if opts.encounters {
            drop_encounters_table(&dbh).await?;
        }
        if opts.tables {
            drop_tables(&dbh).await?;
        }
        if opts.sequences {
            drop_sequences(&dbh).await?;
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
    let opts = &SetupOpts { all: true, ..SetupOpts::default() };
    cleanup_environment(ctx, opts).await?;

    // Fiat Lux
    //
    setup_acute_environment(ctx, opts).await?;

    Ok(())
}
