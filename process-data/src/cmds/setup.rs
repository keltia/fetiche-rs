//! This task connects to the database and create some useful macros and tables
//! to set our work environment up.
//!

use std::env;
use clap::Parser;
use duckdb::Connection;
use eyre::Result;
use tracing::info;

use crate::config::Context;

#[derive(Debug, Default, Parser)]
pub struct SetupOpts {
    /// Add only macros.
    #[clap(short = 'M', long)]
    pub macros: bool,
    /// Create encounters table
    #[clap(short = 'E', long)]
    pub encounters: bool,
    /// Create sequences
    #[clap(short = 'S', long)]
    pub sequences: bool,
    /// Create views
    #[clap(short = 'V', long)]
    pub views: bool,
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
/// - encounter     create a unique ID for encounters
///
#[tracing::instrument(skip(dbh))]
fn add_macros(dbh: &Connection) -> Result<()> {
    info!("Adding macros.");

    let r = r##"
CREATE OR REPLACE MACRO nm_to_deg(nm) AS
  nm * 1.852 / 111111.11;
CREATE OR REPLACE MACRO deg_to_m(deg) AS
  deg * 111111.11;
CREATE OR REPLACE MACRO m_to_deg(m) AS
  m / 111111.11;
CREATE OR REPLACE MACRO dist_2d(px, py, dx, dy) AS
  ST_Distance_Spheroid(ST_Point(px, py), ST_Point(dx, dy));
CREATE OR REPLACE MACRO dist_3d(px, py, pz, dx, dy, dz) AS
  sqrt(pow(dist_2d(px, py, dx, dy), 2) + pow((pz - dz), 2));
CREATE OR REPLACE MACRO encounter(site, tm, journey, id) AS
  printf('%s-%04d%02d%02d_%d_%d', site, datepart('year', CAST(tm AS DATE)), datepart('month', CAST(tm AS DATE)), datepart('day', CAST(tm AS DATE)), journey, id);
    "##;

    Ok(dbh.execute_batch(r)?)
}

#[tracing::instrument(skip(dbh))]
fn remove_macros(dbh: &Connection) -> Result<()> {
    info!("Removing macros.");

    let r = r##"
DROP MACRO IF EXISTS dist_2d;
DROP MACRO IF EXISTS dist_3d;
DROP MACRO IF EXISTS nm_to_deg;
DROP MACRO IF EXISTS deg_to_m;
DROP MACRO IF EXISTS m_to_deg;
DROP MACRO IF EXISTS encounter;
    "##;

    Ok(dbh.execute_batch(r)?)
}

/// Create the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(dbh))]
fn add_encounters_table(dbh: &Connection) -> Result<()> {
    info!("Adding encounters table.");

    let sq = r##"
CREATE OR REPLACE SEQUENCE id_encounter;
CREATE OR REPLACE TABLE encounters (
  id INT DEFAULT nextval('id_encounter'),
  en_id VARCHAR,
  time TIMESTAMP,
  site VARCHAR,
  journey INT, 
  drone_id VARCHAR,
  model VARCHAR,
  callsign VARCHAR, 
  addr VARCHAR, 
  distance FLOAT,
  distancelat FLOAT,
  distancevert FLOAT,
  PRIMARY KEY (time, journey)
)
    "##;

    if dbh
        .execute("SELECT id FROM encounters LIMIT 1", [])
        .is_err()
    {
        // Create sequence & table.
        //
        dbh.execute_batch(sq)?;
    }
    Ok(())
}

/// Remove the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(dbh))]
fn drop_encounters_table(dbh: &Connection) -> Result<()> {
    info!("Removing encounters table.");

    let sq = r##"
DROP TABLE IF EXISTS encounters;
DROP SEQUENCE IF EXISTS id_encounter;
    "##;

    Ok(dbh.execute_batch(sq)?)
}

/// Add the sequences we need
///
#[tracing::instrument]
fn add_sequences(dbh: &Connection) -> Result<()> {
    info!("Adding sequences");

    let seq = r##"
CREATE OR REPLACE SEQUENCE id_encounter;
    "##;

    Ok(dbh.execute_batch(seq)?)
}

/// Add the sequences we need
///
#[tracing::instrument]
fn drop_sequences(dbh: &Connection) -> Result<()> {
    info!("Adding sequences");

    let seq = r##"
DROP SEQUENCE IF EXISTS id_encounter;
    "##;

    Ok(dbh.execute_batch(seq)?)
}

/// Create the two main views
///
/// Assume that the current directory is the datalake so that we use relative paths
/// for `read_parquet()`.
///
#[tracing::instrument(skip(dbh))]
fn create_views(dbh: &Connection) -> Result<()> {
    info!("Creating the airplanes and drones views.");

    let r = r##"
CREATE OR REPLACE VIEW airplanes AS
SELECT *
FROM read_parquet('adsb/**/*.parquet', hive_partitioning = true);
CREATE OR REPLACE VIEW drones
AS (
  SELECT *,
         dist_2d(longitude, latitude, home_lon, home_lat) as home_distance_2d,
         dist_3d(longitude, latitude, height, home_lon, home_lat, home_height) as home_distance_3d
  FROM read_parquet('drones/**/*.parquet')
);
    "##;

    Ok(dbh.execute_batch(r)?)
}

/// Remove both views
///
#[tracing::instrument(skip(dbh))]
fn drop_views(dbh: &Connection) -> Result<()> {
    info!("Dropping airplanes and drones views.");

    let rm = r##"
DROP VIEW IF EXISTS airplanes;
DROP VIEW IF EXISTS drones;
    "##;

    Ok(dbh.execute_batch(rm)?)
}

/// Create parts or all of the ACUTE environment
///
#[tracing::instrument(skip(ctx))]
pub fn setup_acute_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dbh = ctx.db();

    // Move here.
    //
    let _ = env::set_current_dir(&ctx.config["datalake"]);

    if opts.all {
        add_sequences(&dbh)?;
        create_views(&dbh)?;
        add_macros(&dbh)?;
        add_encounters_table(&dbh)?;
    } else {
        if opts.sequences {
            add_sequences(&dbh)?;
        }
        if opts.macros {
            add_macros(&dbh)?;
        }
        if opts.encounters {
            add_encounters_table(&dbh)?;
        }
    }
    Ok(())
}

/// Cleanup by erasing parts or all
///
#[tracing::instrument(skip(ctx))]
pub fn cleanup_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dbh = ctx.db();
    if opts.all {
        remove_macros(&dbh)?;
        drop_encounters_table(&dbh)?;
        remove_macros(&dbh)?;
        drop_views(&dbh)?;
        drop_sequences(&dbh)?;
    } else {
        if opts.macros {
            remove_macros(&dbh)?;
        }
        if opts.encounters {
            drop_encounters_table(&dbh)?;
        }
        if opts.views {
            drop_views(&dbh)?;
        }
        if opts.sequences {
            drop_sequences(&dbh)?;
        }
    }

    Ok(())
}

/// Bootstrapping is a combination of both cleanup/setup to start with a clean slate
///
#[tracing::instrument(skip(ctx))]
pub fn bootstrap(ctx: &Context) -> Result<()> {
    // Remove everything
    //
    let opts = &SetupOpts { all: true, ..SetupOpts::default() };
    cleanup_environment(ctx, opts)?;

    // Fiat Lux
    //
    setup_acute_environment(ctx, opts)?;

    Ok(())
}
